use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub from_symbol: String,
    pub to_symbol: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallGraph {
    pub edges: Vec<CallEdge>,
}

impl CallGraph {
    pub fn add(&mut self, from_symbol: impl Into<String>, to_symbol: impl Into<String>, path: impl Into<String>) {
        self.edges.push(CallEdge {
            from_symbol: from_symbol.into(),
            to_symbol: to_symbol.into(),
            path: path.into(),
        });
    }

    pub fn callers_of(&self, symbol: &str) -> Vec<CallEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.to_symbol.contains(symbol))
            .cloned()
            .collect()
    }

    pub fn calls_from(&self, symbol: &str) -> Vec<CallEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from_symbol.contains(symbol))
            .cloned()
            .collect()
    }
}

pub struct HeuristicCallExtractor;

impl HeuristicCallExtractor {
    pub fn extract(path: &str, source: &str) -> CallGraph {
        let mut graph = CallGraph::default();
        let mut current_fn = "<module>".to_string();

        for line in source.lines() {
            let trimmed = line.trim();

            if let Some(rest) = trimmed.strip_prefix("pub fn ").or_else(|| trimmed.strip_prefix("fn ")) {
                if let Some(name) = rest.split('(').next() {
                    current_fn = name.trim().to_string();
                }
            }

            for token in trimmed.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
                if token.len() > 2 && trimmed.contains(&format!("{}(", token)) && token != current_fn {
                    graph.add(current_fn.clone(), token.to_string(), path.to_string());
                }
            }
        }

        graph
    }
}
