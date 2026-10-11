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

/// Words that carry no search signal when they appear in a free-text goal.
const STOP_TERMS: &[&str] = &["the", "and", "for", "with", "from", "that", "this", "into", "all", "our"];

/// Splits free text (a goal or a query) into lowercase identifier-like terms.
/// Terms shorter than three characters and common stop words are dropped.
pub fn query_terms(text: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for raw in text.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')) {
        let term = raw.to_lowercase();
        if term.chars().count() < 3 || STOP_TERMS.contains(&term.as_str()) {
            continue;
        }
        if !terms.contains(&term) {
            terms.push(term);
        }
    }
    terms
}

/// True when `haystack` (case-insensitive) contains at least one of `terms`.
pub fn contains_any_term(haystack: &str, terms: &[String]) -> bool {
    let lower = haystack.to_lowercase();
    terms.iter().any(|term| lower.contains(term.as_str()))
}

impl SymbolIndex {
    pub fn add(&mut self, symbol: CodeSymbol) {
        self.symbols.push(symbol);
    }

    /// Finds symbols whose name contains at least one term of `query`.
    /// Results are ordered by the number of matching terms (most first).
    /// An empty or term-less query matches nothing.
    pub fn find_by_name(&self, query: &str) -> Vec<CodeSymbol> {
        let terms = query_terms(query);
        if terms.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(usize, &CodeSymbol)> = self
            .symbols
            .iter()
            .filter_map(|symbol| {
                let name = symbol.name.to_lowercase();
                let hits = terms.iter().filter(|term| name.contains(term.as_str())).count();
                (hits > 0).then_some((hits, symbol))
            })
            .collect();

        scored.sort_by_key(|(hits, _)| std::cmp::Reverse(*hits));
        scored.into_iter().map(|(_, symbol)| symbol.clone()).collect()
    }
}
