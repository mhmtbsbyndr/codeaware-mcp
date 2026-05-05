use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Class,
    Method,
    Interface,
    Module,
    Variable,
    Constant,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolTrust {
    Exact,
    Structural,
    Heuristic,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedFile {
    pub path: String,
    pub language: String,
    pub hash: String,
    pub loc: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub id: String,
    pub file_path: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: SymbolKind,
    pub start_line: u64,
    pub end_line: u64,
    pub trust: SymbolTrust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    pub symbol_id: String,
    pub file_path: String,
    pub line: u64,
    pub context: String,
    pub trust: SymbolTrust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallEdge {
    pub caller_symbol_id: String,
    pub callee_symbol_id: String,
    pub file_path: String,
    pub line: u64,
    pub trust: SymbolTrust,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolIndex {
    pub files: BTreeMap<String, IndexedFile>,
    pub symbols: BTreeMap<String, IndexedSymbol>,
    pub references: Vec<SymbolReference>,
    pub call_edges: Vec<CallEdge>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_file(&mut self, file: IndexedFile) {
        self.files.insert(file.path.clone(), file);
    }

    pub fn upsert_symbol(&mut self, symbol: IndexedSymbol) {
        self.symbols.insert(symbol.id.clone(), symbol);
    }

    pub fn add_reference(&mut self, reference: SymbolReference) {
        self.references.push(reference);
    }

    pub fn add_call_edge(&mut self, edge: CallEdge) {
        self.call_edges.push(edge);
    }

    pub fn query_symbols(&self, query: &str) -> Vec<IndexedSymbol> {
        let needle = query.to_lowercase();
        self.symbols
            .values()
            .filter(|symbol| {
                symbol.name.to_lowercase().contains(&needle)
                    || symbol.qualified_name.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    }

    pub fn get_references(&self, symbol_id: &str) -> Vec<SymbolReference> {
        self.references
            .iter()
            .filter(|reference| reference.symbol_id == symbol_id)
            .cloned()
            .collect()
    }

    pub fn get_callers(&self, symbol_id: &str) -> Vec<CallEdge> {
        self.call_edges
            .iter()
            .filter(|edge| edge.callee_symbol_id == symbol_id)
            .cloned()
            .collect()
    }

    pub fn get_callees(&self, symbol_id: &str) -> Vec<CallEdge> {
        self.call_edges
            .iter()
            .filter(|edge| edge.caller_symbol_id == symbol_id)
            .cloned()
            .collect()
    }

    pub fn outline_for_file(&self, file_path: &str) -> Vec<IndexedSymbol> {
        self.symbols
            .values()
            .filter(|symbol| symbol.file_path == file_path)
            .cloned()
            .collect()
    }
}

pub const SYMBOL_INDEX_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS indexed_files (
    path TEXT PRIMARY KEY,
    language TEXT NOT NULL,
    hash TEXT NOT NULL,
    loc INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS indexed_symbols (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    trust TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS symbol_references (
    symbol_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line INTEGER NOT NULL,
    context TEXT NOT NULL,
    trust TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS call_edges (
    caller_symbol_id TEXT NOT NULL,
    callee_symbol_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line INTEGER NOT NULL,
    trust TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_indexed_symbols_name ON indexed_symbols(name);
CREATE INDEX IF NOT EXISTS idx_indexed_symbols_file ON indexed_symbols(file_path);
CREATE INDEX IF NOT EXISTS idx_symbol_references_symbol ON symbol_references(symbol_id);
CREATE INDEX IF NOT EXISTS idx_call_edges_caller ON call_edges(caller_symbol_id);
CREATE INDEX IF NOT EXISTS idx_call_edges_callee ON call_edges(callee_symbol_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(id: &str, name: &str) -> IndexedSymbol {
        IndexedSymbol {
            id: id.to_string(),
            file_path: "src/main.rs".to_string(),
            name: name.to_string(),
            qualified_name: format!("crate::{name}"),
            kind: SymbolKind::Function,
            start_line: 1,
            end_line: 3,
            trust: SymbolTrust::Structural,
        }
    }

    #[test]
    fn queries_symbols_by_name() {
        let mut index = SymbolIndex::new();
        index.upsert_symbol(symbol("s1", "handle_request"));
        index.upsert_symbol(symbol("s2", "build_router"));

        let results = index.query_symbols("request");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "handle_request");
    }

    #[test]
    fn returns_callers_and_callees() {
        let mut index = SymbolIndex::new();
        index.add_call_edge(CallEdge {
            caller_symbol_id: "caller".to_string(),
            callee_symbol_id: "callee".to_string(),
            file_path: "src/main.rs".to_string(),
            line: 10,
            trust: SymbolTrust::Structural,
        });

        assert_eq!(index.get_callers("callee").len(), 1);
        assert_eq!(index.get_callees("caller").len(), 1);
    }

    #[test]
    fn schema_contains_core_tables() {
        assert!(SYMBOL_INDEX_SCHEMA.contains("indexed_files"));
        assert!(SYMBOL_INDEX_SCHEMA.contains("indexed_symbols"));
        assert!(SYMBOL_INDEX_SCHEMA.contains("symbol_references"));
        assert!(SYMBOL_INDEX_SCHEMA.contains("call_edges"));
    }
}
