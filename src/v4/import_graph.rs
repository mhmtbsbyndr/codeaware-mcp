use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEdge {
    pub from_path: String,
    pub import: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportGraph {
    pub edges: Vec<ImportEdge>,
}

impl ImportGraph {
    pub fn add(&mut self, from_path: impl Into<String>, import: impl Into<String>) {
        self.edges.push(ImportEdge {
            from_path: from_path.into(),
            import: import.into(),
        });
    }

    pub fn imports_for_path(&self, path: &str) -> Vec<ImportEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from_path == path)
            .cloned()
            .collect()
    }

    /// File-level edges `(importer, imported)` for imports that resolve to a file in `known`.
    /// External crates and `std` paths are dropped. Output is sorted and de-duplicated.
    pub fn file_edges(&self, known: &BTreeSet<String>) -> Vec<(String, String)> {
        let mut out = BTreeSet::new();
        for edge in &self.edges {
            if let Some(target) = resolve_rust_import(&edge.from_path, &edge.import, known) {
                if target != edge.from_path {
                    out.insert((edge.from_path.clone(), target));
                }
            }
        }
        out.into_iter().collect()
    }
}

/// Resolves one recorded Rust import line to a file path in `known`.
///
/// Handles `crate::a::b::...` (against the `src/` root of `from_path`) and `mod name;`
/// (against the importing file's module directory). Brace groups and globs resolve to
/// their longest existing module prefix.
pub fn resolve_rust_import(from_path: &str, import: &str, known: &BTreeSet<String>) -> Option<String> {
    let text = strip_visibility(import.trim().trim_end_matches(';').trim());

    if let Some(rest) = text.strip_prefix("mod ") {
        let name = leading_ident(rest.trim())?;
        let (dir, stem) = split_dir_stem(from_path);
        let base = if is_module_root(from_path) { dir.to_string() } else { join(dir, stem) };
        return first_known(&[join(&base, &format!("{name}.rs")), join(&join(&base, name), "mod.rs")], known);
    }

    let path = text.strip_prefix("use ")?.trim().strip_prefix("crate::")?;
    let src_root = &from_path[..from_path.find("src/")? + 4];

    let mut segments = Vec::new();
    for segment in path.split("::") {
        let Some(ident) = leading_ident(segment) else {
            break;
        };
        segments.push(ident);
        if segment.contains(['{', '*', ' ']) {
            break;
        }
    }

    (1..=segments.len()).rev().find_map(|len| {
        let module = segments[..len].join("/");
        first_known(
            &[format!("{src_root}{module}.rs"), format!("{src_root}{module}/mod.rs")],
            known,
        )
    })
}

fn first_known(candidates: &[String], known: &BTreeSet<String>) -> Option<String> {
    candidates.iter().find(|candidate| known.contains(candidate.as_str())).cloned()
}

fn leading_ident(text: &str) -> Option<&str> {
    let ident = text.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).next()?;
    (!ident.is_empty()).then_some(ident)
}

fn strip_visibility(text: &str) -> &str {
    let Some(after) = text.strip_prefix("pub") else {
        return text;
    };
    if let Some(inner) = after.strip_prefix('(') {
        match inner.find(')') {
            Some(end) => inner[end + 1..].trim_start(),
            None => text,
        }
    } else if after.starts_with(char::is_whitespace) {
        after.trim_start()
    } else {
        text
    }
}

fn split_dir_stem(path: &str) -> (&str, &str) {
    let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
    (dir, file.strip_suffix(".rs").unwrap_or(file))
}

fn is_module_root(path: &str) -> bool {
    matches!(path.rsplit('/').next(), Some("mod.rs" | "lib.rs" | "main.rs"))
}

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

/// Matches `use`, `mod`, and their visibility-qualified forms
/// (`pub use`, `pub(crate) use`, `pub(super) mod`, ...).
fn is_import_line(trimmed: &str) -> bool {
    let mut rest = trimmed;
    if let Some(after) = rest.strip_prefix("pub") {
        if let Some(inner) = after.strip_prefix('(') {
            match inner.find(')') {
                Some(end) => rest = inner[end + 1..].trim_start(),
                None => return false,
            }
        } else if after.starts_with(char::is_whitespace) {
            rest = after.trim_start();
        }
    }
    rest.starts_with("use ") || rest.starts_with("mod ")
}

pub struct RustImportExtractor;

impl RustImportExtractor {
    pub fn extract(path: &str, source: &str) -> ImportGraph {
        let mut graph = ImportGraph::default();

        for line in source.lines() {
            let trimmed = line.trim();
            if is_import_line(trimmed) {
                graph.add(path, trimmed.trim_end_matches(';'));
            }
        }

        graph
    }
}
