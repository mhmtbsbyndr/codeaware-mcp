//! Deterministic import of SKILL.md files from source repositories into the
//! vendored `skills/` tree. Every candidate is license-checked, scanned, and
//! hashed; only accepted skills are written. Output is sorted, so two runs over
//! the same inputs produce byte-identical files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::frontmatter;
use super::manifest::{Manifest, SkillEntry, SkippedEntry, SourceEntry, SCHEMA_VERSION};
use super::scan::scan_text;
use super::sha256::sha256_hex;
use super::SkillError;

/// Largest SKILL.md accepted.
pub const MAX_SKILL_BYTES: u64 = 256 * 1024;
/// Largest referenced file copied next to a SKILL.md.
pub const MAX_REFERENCED_BYTES: u64 = 64 * 1024;
/// Directory names never descended into.
const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];
/// Directory name of a mirror tree whose skills duplicate the main tree.
const MIRROR_DIR: &str = ".gemini";
const LICENSE_NAMES: &[&str] = &["LICENSE", "LICENSE.txt", "LICENSE.md"];

/// A source repository this crate knows how to attribute.
#[derive(Debug, Clone, Copy)]
pub struct KnownSource {
    pub id: &'static str,
    pub repo: &'static str,
    /// License used when a skill's directory chain has no LICENSE file.
    pub default_license: &'static str,
}

pub const KNOWN_SOURCES: &[KnownSource] = &[
    KnownSource {
        id: "alirezarezvani-claude-skills",
        repo: "https://github.com/alirezarezvani/claude-skills",
        default_license: "MIT",
    },
    KnownSource {
        id: "multica-karpathy-skills",
        repo: "https://github.com/multica-ai/andrej-karpathy-skills",
        default_license: "MIT",
    },
    KnownSource {
        id: "composiohq-awesome-claude-skills",
        repo: "https://github.com/ComposioHQ/awesome-claude-skills",
        default_license: "Apache-2.0",
    },
];

/// One source checkout to import.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub id: String,
    pub repo: String,
    pub root: PathBuf,
    pub commit: String,
    pub default_license: Option<String>,
}

/// Outcome of a single candidate.
enum Outcome {
    Accepted(Accepted),
    Skipped { reason: String, detail: String },
}

struct Accepted {
    id: String,
    source_path: String,
    skill_md: Vec<u8>,
    files: Vec<(String, Vec<u8>)>,
    license: String,
    license_rel: Option<String>,
}

#[derive(Debug, Clone)]
struct Candidate {
    /// Directory containing SKILL.md, relative to the source root, `/`-separated.
    rel_dir: String,
    mirror: bool,
}

impl Candidate {
    fn basename(&self) -> &str {
        self.rel_dir.rsplit('/').next().unwrap_or(&self.rel_dir)
    }
}

/// Imports every source into `out`, replacing each source's previous subtree,
/// and writes `MANIFEST.toml` and `NOTICE.md` at the top of `out`.
pub fn import(sources: &[SourceSpec], out: &Path) -> Result<Manifest, SkillError> {
    let mut ordered: Vec<&SourceSpec> = sources.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));

    fs::create_dir_all(out).map_err(|source| io_err(out, source))?;

    let mut manifest = Manifest {
        schema: SCHEMA_VERSION,
        sources: Vec::new(),
        skills: Vec::new(),
        skipped: Vec::new(),
    };

    for spec in ordered {
        manifest.sources.push(SourceEntry {
            id: spec.id.clone(),
            repo: spec.repo.clone(),
            commit: spec.commit.clone(),
            default_license: spec.default_license.clone(),
        });
        let (skills, skipped) = import_source(spec, out)?;
        manifest.skills.extend(skills);
        manifest.skipped.extend(skipped);
    }

    manifest.skills.sort_by(|a, b| (&a.source, &a.id).cmp(&(&b.source, &b.id)));
    manifest
        .skipped
        .sort_by(|a, b| (&a.source, &a.source_path).cmp(&(&b.source, &b.source_path)));

    fs::write(out.join(super::manifest::MANIFEST_FILE), manifest.to_toml()?)
        .map_err(|source| io_err(out, source))?;
    fs::write(out.join("NOTICE.md"), notice(&manifest)).map_err(|source| io_err(out, source))?;
    Ok(manifest)
}

fn import_source(
    spec: &SourceSpec,
    out: &Path,
) -> Result<(Vec<SkillEntry>, Vec<SkippedEntry>), SkillError> {
    let mut candidates = Vec::new();
    discover(&spec.root, &spec.root, 0, &mut candidates)?;
    candidates.sort_by(|a, b| a.rel_dir.cmp(&b.rel_dir));

    // Non-mirror basenames decide which mirror copies are duplicates.
    let main_names: BTreeSet<&str> = candidates
        .iter()
        .filter(|c| !c.mirror)
        .map(Candidate::basename)
        .collect();
    // Basenames shared by two main-tree skills get path-based ids.
    let mut name_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for c in candidates.iter().filter(|c| !c.mirror) {
        *name_counts.entry(c.basename()).or_insert(0) += 1;
    }

    let mut skipped = Vec::new();
    let mut accepted: Vec<Accepted> = Vec::new();
    let mut used_ids: BTreeSet<String> = BTreeSet::new();
    let mut license_copies: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    for cand in &candidates {
        let source_path = format!("{}/SKILL.md", cand.rel_dir);
        if cand.mirror && main_names.contains(cand.basename()) {
            skipped.push(skip(spec, &source_path, "gemini-mirror-duplicate", "same name as a main-tree skill"));
            continue;
        }
        let id = if name_counts.get(cand.basename()).copied().unwrap_or(0) > 1 && !cand.mirror {
            slug(&cand.rel_dir)
        } else {
            slug(cand.basename())
        };
        if id.is_empty() || !used_ids.insert(id.clone()) {
            skipped.push(skip(spec, &source_path, "duplicate-id", &id));
            continue;
        }
        match evaluate(spec, cand, &id) {
            Outcome::Accepted(mut acc) => {
                acc.source_path = source_path;
                if let Some(rel) = &acc.license_rel {
                    if !license_copies.contains_key(rel) {
                        let bytes = fs::read(spec.root.join(rel)).map_err(|s| io_err(&spec.root.join(rel), s))?;
                        license_copies.insert(rel.clone(), bytes);
                    }
                }
                accepted.push(acc);
            }
            Outcome::Skipped { reason, detail } => {
                used_ids.remove(&id);
                skipped.push(skip(spec, &source_path, &reason, &detail));
            }
        }
    }

    // Replace this source's subtree so stale files cannot survive a re-import.
    let source_out = out.join(&spec.id);
    if source_out.exists() {
        fs::remove_dir_all(&source_out).map_err(|s| io_err(&source_out, s))?;
    }
    fs::create_dir_all(&source_out).map_err(|s| io_err(&source_out, s))?;

    // Root LICENSE is always vendored for attribution, when the repo has one.
    if let Some(root_license) = LICENSE_NAMES.iter().find(|n| spec.root.join(n).is_file()) {
        let bytes = fs::read(spec.root.join(root_license)).map_err(|s| io_err(&spec.root, s))?;
        license_copies.entry((*root_license).to_string()).or_insert(bytes);
    }
    if !license_copies.is_empty() {
        let dir = source_out.join("LICENSES");
        fs::create_dir_all(&dir).map_err(|s| io_err(&dir, s))?;
        for (rel, bytes) in &license_copies {
            let dest = dir.join(format!("{}.txt", slug(rel)));
            fs::write(&dest, bytes).map_err(|s| io_err(&dest, s))?;
        }
    }

    let mut entries = Vec::new();
    for acc in accepted {
        let skill_dir = source_out.join(&acc.id);
        fs::create_dir_all(&skill_dir).map_err(|s| io_err(&skill_dir, s))?;
        fs::write(skill_dir.join("SKILL.md"), &acc.skill_md).map_err(|s| io_err(&skill_dir, s))?;
        for (rel, bytes) in &acc.files {
            let dest = skill_dir.join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|s| io_err(parent, s))?;
            }
            fs::write(&dest, bytes).map_err(|s| io_err(&dest, s))?;
        }
        entries.push(SkillEntry {
            id: acc.id.clone(),
            source: spec.id.clone(),
            source_path: acc.source_path.clone(),
            path: format!("{}/{}/SKILL.md", spec.id, acc.id),
            license: acc.license.clone(),
            license_file: acc
                .license_rel
                .as_ref()
                .map(|rel| format!("{}/LICENSES/{}.txt", spec.id, slug(rel))),
            sha256: sha256_hex(&acc.skill_md),
            files: acc.files.iter().map(|(rel, _)| rel.clone()).collect(),
        });
    }
    Ok((entries, skipped))
}

fn evaluate(spec: &SourceSpec, cand: &Candidate, id: &str) -> Outcome {
    let dir = spec.root.join(&cand.rel_dir);
    let skill_path = dir.join("SKILL.md");
    let skip_with = |reason: &str, detail: String| Outcome::Skipped {
        reason: reason.to_string(),
        detail,
    };

    match fs::symlink_metadata(&skill_path) {
        Ok(meta) if meta.is_file() && meta.len() <= MAX_SKILL_BYTES => {}
        Ok(meta) if meta.len() > MAX_SKILL_BYTES => return skip_with("too-large", format!("{} bytes", meta.len())),
        _ => return skip_with("unreadable", "SKILL.md is not a regular file".into()),
    }
    let skill_md = match fs::read(&skill_path) {
        Ok(b) => b,
        Err(e) => return skip_with("unreadable", e.to_string()),
    };
    let text = match std::str::from_utf8(&skill_md) {
        Ok(t) => t,
        Err(_) => return skip_with("not-utf8", String::new()),
    };

    let (license, license_rel) = match resolve_license(spec, &dir) {
        Ok(found) => found,
        Err(reason) => return skip_with(reason.0, reason.1),
    };
    let Some(license) = license.or_else(|| spec.default_license.clone()) else {
        return skip_with("no-license", "no LICENSE file and no default".into());
    };

    if let Some(f) = scan_text(text) {
        return skip_with(
            &format!("security:{}", f.rule.as_str()),
            format!("SKILL.md:{} {}", f.line, f.snippet),
        );
    }

    let mut files = Vec::new();
    for rel in referenced_files(&frontmatter::split(text).1) {
        let path = dir.join(&rel);
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if !meta.is_file() || meta.len() > MAX_REFERENCED_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let Ok(content) = std::str::from_utf8(&bytes) else { continue };
        if let Some(f) = scan_text(content) {
            return skip_with(
                &format!("security:{}", f.rule.as_str()),
                format!("{}:{} {}", rel, f.line, f.snippet),
            );
        }
        files.push((rel, bytes));
    }

    Outcome::Accepted(Accepted {
        id: id.to_string(),
        source_path: String::new(),
        skill_md,
        files,
        license,
        license_rel,
    })
}

/// License of the nearest LICENSE file from the skill directory up to the
/// source root. Returns `(spdx, root-relative license path)`, or `(None, None)`
/// when there is no license file.
#[allow(clippy::type_complexity)]
fn resolve_license(
    spec: &SourceSpec,
    skill_dir: &Path,
) -> Result<(Option<String>, Option<String>), (&'static str, String)> {
    let mut dir = Some(skill_dir);
    while let Some(d) = dir {
        for name in LICENSE_NAMES {
            let path = d.join(name);
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.is_file() {
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    let rel = path
                        .strip_prefix(&spec.root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    return match classify_license(&text) {
                        Ok(spdx) => Ok((Some(spdx.to_string()), Some(rel))),
                        Err(reason) => Err((reason, rel)),
                    };
                }
            }
        }
        if d == spec.root {
            break;
        }
        dir = d.parent();
    }
    Ok((None, None))
}

/// Classifies a LICENSE text. Only plain MIT and Apache-2.0 grants pass. Text
/// that reserves rights or forbids redistribution is a conflict. Anything else
/// is unclear and skipped.
pub fn classify_license(text: &str) -> Result<&'static str, &'static str> {
    let reserved = text.contains("All rights reserved")
        || text.contains("Distribute, sublicense, or transfer");
    let mit = text.contains("MIT License") && text.contains("Permission is hereby granted");
    let apache = text.contains("Apache License") && text.contains("Version 2.0");
    if reserved && !(mit || apache) {
        return Err("license-conflict");
    }
    if apache && !mit {
        return Ok("Apache-2.0");
    }
    if mit && !apache {
        return Ok("MIT");
    }
    Err("license-unclear")
}

/// Relative paths written as markdown link targets or backtick code spans that
/// look like local files. Absolute paths, URLs, parent references and anchors
/// are ignored. The caller checks that each path exists.
pub fn referenced_files(body: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    let mut candidates: Vec<String> = Vec::new();
    // Markdown link targets: `](target)`.
    for part in body.split("](").skip(1) {
        if let Some(end) = part.find(')') {
            candidates.push(part[..end].split_whitespace().next().unwrap_or("").to_string());
        }
    }
    // Backtick spans.
    for part in body.split('`').skip(1).step_by(2) {
        candidates.push(part.trim().to_string());
    }
    for raw in candidates {
        let target = raw.trim().trim_start_matches("./");
        if target.is_empty()
            || target.contains("://")
            || target.contains(':')
            || target.starts_with('/')
            || target.starts_with('#')
            || target.contains('?')
            || target.contains('*')
            || target.split('/').any(|seg| seg == "..")
            || target.contains(' ')
        {
            continue;
        }
        let Some((_, ext)) = target.rsplit_once('.') else { continue };
        if target.ends_with('/') || target.eq_ignore_ascii_case("SKILL.md") {
            continue;
        }
        if ext.is_empty() || ext.len() > 8 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        out.insert(target.to_string());
    }
    out.into_iter().collect()
}

fn discover(root: &Path, dir: &Path, depth: usize, out: &mut Vec<Candidate>) -> Result<(), SkillError> {
    if depth > 12 {
        return Ok(());
    }
    if dir.join("SKILL.md").is_file() {
        let rel = dir
            .strip_prefix(root)
            .unwrap_or(dir)
            .to_string_lossy()
            .replace('\\', "/");
        if !rel.is_empty() {
            let mirror = rel.split('/').any(|seg| seg == MIRROR_DIR);
            out.push(Candidate { rel_dir: rel, mirror });
        }
    }
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|s| io_err(dir, s))? {
        let entry = entry.map_err(|s| io_err(dir, s))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP_DIRS.contains(&name.as_ref()) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if meta.is_dir() {
            children.push(path);
        }
    }
    children.sort();
    for child in children {
        discover(root, &child, depth + 1, out)?;
    }
    Ok(())
}

/// Lowercase, `[a-z0-9_-]` identifier. Path separators and dots become `-`.
pub fn slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut dash = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            dash = c == '-';
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn skip(spec: &SourceSpec, source_path: &str, reason: &str, detail: &str) -> SkippedEntry {
    SkippedEntry {
        source: spec.id.clone(),
        source_path: source_path.to_string(),
        reason: reason.to_string(),
        detail: detail.to_string(),
    }
}

fn io_err(path: &Path, source: std::io::Error) -> SkillError {
    SkillError::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Deterministic NOTICE.md text: one section per source with repo, commit,
/// license, and counts.
pub fn notice(manifest: &Manifest) -> String {
    let mut out = String::new();
    out.push_str("# Third-party skill notices\n\n");
    out.push_str(
        "The files under `skills/<source>/` are unmodified copies of SKILL.md files and the small \
         files they reference, taken from the open-source repositories below. The exact commit and \
         a SHA-256 hash for every vendored SKILL.md are in `MANIFEST.toml`. Copyright notices and \
         license texts are in each source's `LICENSES/` directory. Skills with a conflicting or \
         unclear license, and skills that failed the security scan, are not vendored. They are \
         listed in `MANIFEST.toml` under `[[skipped]]`.\n\n",
    );
    for src in &manifest.sources {
        out.push_str(&format!("## {}\n\n", src.id));
        out.push_str(&format!("- Repository: {}\n", src.repo));
        out.push_str(&format!("- Pinned commit: {}\n", src.commit));
        let imported: Vec<&SkillEntry> = manifest.skills.iter().filter(|s| s.source == src.id).collect();
        let skipped = manifest.skipped.iter().filter(|s| s.source == src.id).count();
        out.push_str(&format!("- Imported skills: {}\n", imported.len()));
        out.push_str(&format!("- Skipped entries: {skipped}\n"));
        let mut by_license: BTreeMap<&str, usize> = BTreeMap::new();
        for s in &imported {
            *by_license.entry(s.license.as_str()).or_insert(0) += 1;
        }
        for (lic, n) in by_license {
            out.push_str(&format!("- License {lic}: {n} skill(s)\n"));
        }
        if let Some(default) = &src.default_license {
            out.push_str(&format!(
                "- Default license when no LICENSE file applies: {default}\n"
            ));
        }
        let root_license = format!("{}/LICENSES/LICENSE.txt", src.id);
        out.push_str(&format!(
            "- Repository license text: `{}` (present only when the repository ships one)\n\n",
            root_license
        ));
    }
    out
}
