use serde::{Deserialize, Serialize};

use crate::v4::call_graph::CallGraph;
use crate::v4::import_graph::ImportGraph;
use crate::v4::symbols::{CodeSymbol, SymbolIndex};
use crate::v4::tests_graph::TestGraph;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticIndex {
    pub symbols: SymbolIndex,
    pub imports: ImportGraph,
    pub calls: CallGraph,
    pub tests: TestGraph,
}

impl SemanticIndex {
    pub fn symbol_count(&self) -> usize {
        self.symbols.symbols.len()
    }

    pub fn import_count(&self) -> usize {
        self.imports.edges.len()
    }

    pub fn call_count(&self) -> usize {
        self.calls.edges.len()
    }

    pub fn test_count(&self) -> usize {
        self.tests.references.len()
    }

    pub fn find_symbols(&self, query: &str) -> Vec<CodeSymbol> {
        self.symbols.find_by_name(query)
    }
}
