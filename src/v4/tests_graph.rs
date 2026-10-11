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

    /// Tests whose referenced name is exactly `symbol`. Empty query returns nothing.
    pub fn find_tests_for_symbol(&self, symbol: &str) -> Vec<TestReference> {
        if symbol.is_empty() {
            return Vec::new();
        }
        self.references
            .iter()
            .filter(|reference| reference.referenced_symbol == symbol)
            .cloned()
            .collect()
    }
}

/// Returns true when the attribute line (`#[...]`) names a test attribute:
/// `#[test]`, `#[tokio::test]`, `#[async_std::test]`, ...
fn is_test_attribute(trimmed: &str) -> bool {
    let Some(inner) = trimmed.strip_prefix("#[") else {
        return false;
    };
    let path = inner
        .split(|ch: char| ch == ']' || ch == '(' || ch.is_whitespace())
        .next()
        .unwrap_or("");
    path.rsplit("::").next() == Some("test")
}

pub struct RustTestExtractor;

impl RustTestExtractor {
    /// Detects test functions as functions declared directly under a test attribute.
    /// Only the function name is stored as the referenced symbol.
    pub fn extract(path: &str, source: &str) -> TestGraph {
        let mut graph = TestGraph::default();
        let mut pending_test_attr = false;

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("#[") {
                if is_test_attribute(trimmed) {
                    pending_test_attr = true;
                }
                continue;
            }

            if pending_test_attr {
                if let Some(name) = fn_name_of(trimmed) {
                    graph.add(path, name);
                }
            }
            pending_test_attr = false;
        }

        graph
    }
}

fn fn_name_of(trimmed: &str) -> Option<&str> {
    let mut rest = trimmed;
    if let Some(after) = rest.strip_prefix("pub") {
        if after.starts_with(char::is_whitespace) {
            rest = after.trim_start();
        }
    }
    if let Some(after) = rest.strip_prefix("async") {
        if after.starts_with(char::is_whitespace) {
            rest = after.trim_start();
        }
    }
    let after_fn = rest.strip_prefix("fn")?;
    if !after_fn.starts_with(char::is_whitespace) {
        return None;
    }
    let name = after_fn
        .trim_start()
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .next()?;
    (!name.is_empty()).then_some(name)
}
