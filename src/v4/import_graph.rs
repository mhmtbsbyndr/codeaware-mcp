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
