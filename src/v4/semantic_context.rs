use serde::{Deserialize, Serialize};

use crate::v4::context_items::{ContextItem, ContextItemKind};
use crate::v4::semantic_index::SemanticIndex;
use crate::v4::symbols::{contains_any_term, query_terms};
use crate::v4::tokens::estimate_tokens;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContextOptions {
    pub max_symbols: usize,
    pub max_imports: usize,
    pub max_calls: usize,
    pub max_tests: usize,
}

impl Default for SemanticContextOptions {
    fn default() -> Self {
        Self {
            max_symbols: 12,
            max_imports: 12,
            max_calls: 12,
            max_tests: 12,
        }
    }
}

pub struct SemanticContextAssembler;

impl SemanticContextAssembler {
    /// Assembles context items that relate to the goal. Only symbols, imports, calls and
    /// tests whose names or paths contain a goal term are included, so an unrelated goal
    /// yields no items and callers can fall back to file summaries.
    pub fn assemble(goal: &str, index: &SemanticIndex, options: SemanticContextOptions) -> Vec<ContextItem> {
        let mut items = Vec::new();
        let terms = query_terms(goal);
        if terms.is_empty() {
            return items;
        }

        for symbol in index.find_symbols(goal).into_iter().take(options.max_symbols) {
            let content = format!(
                "Symbol: {:?} {} in {}:{}-{}",
                symbol.kind, symbol.name, symbol.path, symbol.line_start, symbol.line_end
            );
            items.push(ContextItem {
                kind: ContextItemKind::FileExcerpt,
                path: Some(symbol.path),
                symbol: Some(symbol.name),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Matched semantic symbol query.".to_string(),
            });
        }

        let relevant_imports = index
            .imports
            .edges
            .iter()
            .filter(|edge| contains_any_term(&edge.from_path, &terms) || contains_any_term(&edge.import, &terms));
        for edge in relevant_imports.take(options.max_imports) {
            let content = format!("Import: {} -> {}", edge.from_path, edge.import);
            items.push(ContextItem {
                kind: ContextItemKind::RecentChange,
                path: Some(edge.from_path.clone()),
                symbol: None,
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Import relationship from semantic index.".to_string(),
            });
        }

        let relevant_calls = index.calls.edges.iter().filter(|edge| {
            contains_any_term(&edge.path, &terms)
                || contains_any_term(&edge.from_symbol, &terms)
                || contains_any_term(&edge.to_symbol, &terms)
        });
        for edge in relevant_calls.take(options.max_calls) {
            let content = format!("Call: {} -> {} in {}", edge.from_symbol, edge.to_symbol, edge.path);
            items.push(ContextItem {
                kind: ContextItemKind::FileExcerpt,
                path: Some(edge.path.clone()),
                symbol: Some(edge.from_symbol.clone()),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Call relationship from semantic index.".to_string(),
            });
        }

        let relevant_tests = index.tests.references.iter().filter(|test| {
            contains_any_term(&test.test_path, &terms) || contains_any_term(&test.referenced_symbol, &terms)
        });
        for test in relevant_tests.take(options.max_tests) {
            let content = format!("Test: {} references {}", test.test_path, test.referenced_symbol);
            items.push(ContextItem {
                kind: ContextItemKind::TestHint,
                path: Some(test.test_path.clone()),
                symbol: Some(test.referenced_symbol.clone()),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Test relationship from semantic index.".to_string(),
            });
        }

        items
    }
}
