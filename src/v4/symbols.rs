use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Constant,
    TypeAlias,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolIndex {
    pub symbols: Vec<CodeSymbol>,
}

impl SymbolIndex {
    pub fn add(&mut self, symbol: CodeSymbol) {
        self.symbols.push(symbol);
    }

    pub fn find_by_name(&self, query: &str) -> Vec<CodeSymbol> {
        let needle = query.to_lowercase();
        self.symbols
            .iter()
            .filter(|symbol| symbol.name.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }
}
