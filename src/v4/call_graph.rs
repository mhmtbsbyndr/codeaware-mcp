use std::collections::{BTreeMap, BTreeSet};

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

    /// Edges whose callee is exactly `symbol`. Empty query returns nothing.
    pub fn callers_of(&self, symbol: &str) -> Vec<CallEdge> {
        if symbol.is_empty() {
            return Vec::new();
        }
        self.edges
            .iter()
            .filter(|edge| edge.to_symbol == symbol)
            .cloned()
            .collect()
    }

    /// Edges whose caller is exactly `symbol`. Empty query returns nothing.
    pub fn calls_from(&self, symbol: &str) -> Vec<CallEdge> {
        if symbol.is_empty() {
            return Vec::new();
        }
        self.edges
            .iter()
            .filter(|edge| edge.from_symbol == symbol)
            .cloned()
            .collect()
    }
}

/// Keywords and builtins that look like calls (`if (`, `match(`) but are not.
const NON_CALL_WORDS: &[&str] = &[
    "if", "while", "match", "for", "loop", "return", "fn", "pub", "impl", "where", "move", "async",
    "await", "unsafe", "const", "let", "mut", "else", "Some", "Ok", "Err", "self", "Self", "super", "crate",
];

/// Returns the function name declared on this line, if the line starts a function.
/// Recognises `fn`, `pub fn`, `pub(crate) fn`, `pub(super) fn`, `async fn`, `const fn`,
/// `unsafe fn`, `extern "C" fn` and combinations. Generics are stripped (`foo<T>` -> `foo`).
fn declared_fn_name(trimmed: &str) -> Option<&str> {
    let mut rest = trimmed;
    loop {
        if let Some(after) = rest.strip_prefix("pub") {
            if let Some(inner) = after.strip_prefix('(') {
                let end = inner.find(')')?;
                rest = inner[end + 1..].trim_start();
                continue;
            }
            if after.starts_with(char::is_whitespace) {
                rest = after.trim_start();
                continue;
            }
        }

        let mut advanced = false;
        for keyword in ["async", "const", "unsafe", "extern"] {
            if let Some(after) = rest.strip_prefix(keyword) {
                if after.starts_with(char::is_whitespace) {
                    rest = after.trim_start();
                    advanced = true;
                    break;
                }
            }
        }
        if advanced {
            // `extern "C" fn`: skip the ABI string literal.
            if let Some(after_quote) = rest.strip_prefix('"') {
                let end = after_quote.find('"')?;
                rest = after_quote[end + 1..].trim_start();
            }
            continue;
        }
        break;
    }

    let after_fn = rest.strip_prefix("fn")?;
    if !after_fn.starts_with(char::is_whitespace) {
        return None;
    }
    let name = after_fn
        .trim_start()
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Yields identifiers that are immediately followed by `(` on this line.
/// Identifier tokens are maximal runs, so `xtotal(` never yields `total`.
fn call_candidates(line: &str) -> impl Iterator<Item = &str> {
    let bytes = line.as_bytes();
    line.split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .filter(move |token| {
            let start = token.as_ptr() as usize - line.as_ptr() as usize;
            let end = start + token.len();
            bytes.get(end) == Some(&b'(')
        })
}

pub struct HeuristicCallExtractor;

impl HeuristicCallExtractor {
    pub fn extract(path: &str, source: &str) -> CallGraph {
        let mut graph = CallGraph::default();
        let mut current_fn = "<module>".to_string();

        for line in source.lines() {
            let trimmed = line.trim();

            if let Some(name) = declared_fn_name(trimmed) {
                current_fn = name.to_string();
            }

            for token in call_candidates(trimmed) {
                if token.len() > 2
                    && !NON_CALL_WORDS.contains(&token)
                    && token != current_fn
                    && !token.chars().next().is_some_and(|ch| ch.is_ascii_digit())
                {
                    graph.add(current_fn.clone(), token.to_string(), path.to_string());
                }
            }
        }

        graph
    }
}
