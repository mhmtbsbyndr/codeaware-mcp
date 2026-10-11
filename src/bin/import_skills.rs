//! Imports third-party SKILL.md files into the vendored `skills/` tree.
//!
//! Usage:
//!   import_skills --source <id>=<checkout-path> [--source ...] [--out <dir>]
//!
//! Known source ids: alirezarezvani-claude-skills, multica-karpathy-skills,
//! composiohq-awesome-claude-skills. Each checkout must be a git work tree; its
//! HEAD commit is pinned in the manifest.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use codeaware_mcp::skills::import::{self, SourceSpec, KNOWN_SOURCES};

const USAGE: &str = "\
import_skills: vendor SKILL.md files from pinned source checkouts.

USAGE:
    import_skills --source <id>=<path> [--source <id>=<path> ...] [--out <dir>]

OPTIONS:
    --source <id>=<path>   Import one checkout. Repeatable. Required.
    --out <dir>            Output directory (default: skills).
    -h, --help             Print this help.

KNOWN SOURCE IDS:
    alirezarezvani-claude-skills   https://github.com/alirezarezvani/claude-skills
    multica-karpathy-skills        https://github.com/multica-ai/andrej-karpathy-skills
    composiohq-awesome-claude-skills
                                   https://github.com/ComposioHQ/awesome-claude-skills

Writes <out>/<source>/... , <out>/MANIFEST.toml and <out>/NOTICE.md.
Output is deterministic: the same checkouts produce identical files.
";

struct Args {
    sources: Vec<(String, PathBuf)>,
    out: PathBuf,
}

fn parse_args(mut raw: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut sources = Vec::new();
    let mut out = PathBuf::from("skills");
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--source" => {
                let value = raw.next().ok_or("--source needs <id>=<path>")?;
                let (id, path) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--source expects <id>=<path>, got '{value}'"))?;
                sources.push((id.to_string(), PathBuf::from(path)));
            }
            "--out" => {
                out = PathBuf::from(raw.next().ok_or("--out needs a directory")?);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if sources.is_empty() {
        return Err("at least one --source is required".into());
    }
    Ok(Some(Args { sources, out }))
}

fn head_commit(path: &PathBuf) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("cannot run git for {}: {e}", path.display()))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse HEAD failed in {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run() -> Result<(), String> {
    let args = match parse_args(std::env::args().skip(1))? {
        Some(args) => args,
        None => {
            print!("{USAGE}");
            return Ok(());
        }
    };

    let mut specs = Vec::new();
    for (id, root) in &args.sources {
        let known = KNOWN_SOURCES
            .iter()
            .find(|k| k.id == id)
            .ok_or_else(|| format!("unknown source id '{id}'"))?;
        let commit = head_commit(root)?;
        specs.push(SourceSpec {
            id: known.id.to_string(),
            repo: known.repo.to_string(),
            root: root.clone(),
            commit,
            default_license: Some(known.default_license.to_string()),
        });
    }

    let manifest = import::import(&specs, &args.out).map_err(|e| e.to_string())?;

    for src in &manifest.sources {
        let imported = manifest
            .skills
            .iter()
            .filter(|s| s.source == src.id)
            .count();
        let skipped: Vec<_> = manifest
            .skipped
            .iter()
            .filter(|s| s.source == src.id)
            .collect();
        println!("{}: imported {imported}, skipped {}", src.id, skipped.len());
        let mut reasons = std::collections::BTreeMap::new();
        for s in &skipped {
            *reasons.entry(s.reason.as_str()).or_insert(0usize) += 1;
        }
        for (reason, n) in reasons {
            println!("    {reason}: {n}");
        }
    }
    println!("manifest: {}", args.out.join("MANIFEST.toml").display());
    println!("notice:   {}", args.out.join("NOTICE.md").display());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("import_skills: {message}");
            eprintln!("run with --help for usage");
            ExitCode::FAILURE
        }
    }
}
