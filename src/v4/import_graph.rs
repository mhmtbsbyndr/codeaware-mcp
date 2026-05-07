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

pub struct RustImportExtractor;

impl RustImportExtractor {
    pub fn extract(path: &str, source: &str) -> ImportGraph {
        let mut graph = ImportGraph::default();

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") || trimmed.starts_with("mod ") {
                graph.add(path, trimmed.trim_end_matches(';'));
            }
        }

        graph
    }
}
