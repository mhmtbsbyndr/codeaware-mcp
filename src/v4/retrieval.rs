use serde::{Deserialize, Serialize};

use crate::v4::symbols::{CodeSymbol, SymbolIndex};
use crate::v4::tests_graph::{TestGraph, TestReference};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSearchResult {
    pub query: String,
    pub matches: Vec<CodeSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTestResult {
    pub symbol: String,
    pub tests: Vec<TestReference>,
}

pub struct SemanticRetrieval;

impl SemanticRetrieval {
    pub fn find_symbol(index: &SymbolIndex, query: &str) -> SymbolSearchResult {
        SymbolSearchResult {
            query: query.to_string(),
            matches: index.find_by_name(query),
        }
    }

    pub fn find_tests(test_graph: &TestGraph, symbol: &str) -> SymbolTestResult {
        SymbolTestResult {
            symbol: symbol.to_string(),
            tests: test_graph.find_tests_for_symbol(symbol),
        }
    }
}
