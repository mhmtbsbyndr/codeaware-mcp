use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReference {
    pub test_path: String,
    pub referenced_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestGraph {
    pub references: Vec<TestReference>,
}

impl TestGraph {
    pub fn add(&mut self, test_path: impl Into<String>, referenced_symbol: impl Into<String>) {
        self.references.push(TestReference {
            test_path: test_path.into(),
            referenced_symbol: referenced_symbol.into(),
        });
    }

    pub fn find_tests_for_symbol(&self, symbol: &str) -> Vec<TestReference> {
        self.references
            .iter()
            .filter(|reference| reference.referenced_symbol.contains(symbol))
            .cloned()
            .collect()
    }
}

pub struct RustTestExtractor;

impl RustTestExtractor {
    pub fn extract(path: &str, source: &str) -> TestGraph {
        let mut graph = TestGraph::default();

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("fn test_") {
                graph.add(path, trimmed.to_string());
            }
        }

        graph
    }
}
