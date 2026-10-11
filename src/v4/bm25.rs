//! Pure-Rust BM25 with identifier splitting (camelCase, snake_case, digit runs).
//!
//! Documents are bags of weighted text fields (paths, symbol names, doc lines). Field
//! weights multiply term frequencies, so a hit in a path can count more than a hit in a
//! doc comment. Everything is deterministic: terms live in `BTreeMap`s and ties are broken
//! by the lower document id.

use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_K1: f64 = 1.2;
pub const DEFAULT_B: f64 = 0.75;

/// Query-side words that carry no retrieval signal. Document-side tokens are kept as-is;
/// IDF already down-weights them.
const QUERY_STOP_TERMS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "into", "all", "our", "are", "not",
    "how", "what", "why", "when", "where", "should", "can", "add", "fix", "use", "make", "does",
    "its", "it", "of", "to", "in", "on", "by", "or", "an", "a", "is", "be", "as", "at", "we",
    "you", "if", "so", "do", "rs", "md", "src", "mod", "fn",
];

/// Splits text into lowercase identifier parts.
///
/// Non-alphanumeric characters separate tokens. Inside a token a boundary is placed at
/// `fooBar` (lower to upper), `HTTPServer` (upper run followed by a capitalised word) and
/// between letters and digits (`bm25` -> `bm`, `25`). `snake_case` splits on `_`.
pub fn split_identifiers(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    for raw in text.split(|ch: char| !ch.is_alphanumeric()) {
        if raw.is_empty() {
            continue;
        }
        let chars: Vec<char> = raw.chars().collect();
        let mut start = 0;
        for i in 1..chars.len() {
            let prev = chars[i - 1];
            let cur = chars[i];
            let next = chars.get(i + 1).copied();
            let boundary = (prev.is_lowercase() && cur.is_uppercase())
                || (prev.is_uppercase() && cur.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
                || (prev.is_ascii_digit() != cur.is_ascii_digit());
            if boundary {
                parts.push(chars[start..i].iter().collect::<String>().to_lowercase());
                start = i;
            }
        }
        parts.push(chars[start..].iter().collect::<String>().to_lowercase());
    }
    parts
}

/// Unique query terms in first-seen order. Single characters and stop words are dropped.
pub fn query_tokens(query: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut terms = Vec::new();
    for token in split_identifiers(query) {
        if token.chars().count() < 2 || QUERY_STOP_TERMS.contains(&token.as_str()) {
            continue;
        }
        if seen.insert(token.clone()) {
            terms.push(token);
        }
    }
    terms
}

#[derive(Debug, Clone)]
pub struct Bm25Index {
    k1: f64,
    b: f64,
    doc_len: Vec<f64>,
    total_len: f64,
    postings: BTreeMap<String, Vec<(usize, f64)>>,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Index {
    pub fn new() -> Self {
        Self::with_params(DEFAULT_K1, DEFAULT_B)
    }

    pub fn with_params(k1: f64, b: f64) -> Self {
        Self {
            k1,
            b,
            doc_len: Vec::new(),
            total_len: 0.0,
            postings: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.doc_len.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doc_len.is_empty()
    }

    /// Adds a document made of `(text, weight)` fields and returns its id (insertion index).
    /// Fields with a non-positive weight are ignored.
    pub fn add_document(&mut self, fields: &[(&str, f64)]) -> usize {
        let id = self.doc_len.len();
        let mut tf: BTreeMap<String, f64> = BTreeMap::new();
        let mut len = 0.0;
        for (text, weight) in fields {
            if *weight <= 0.0 {
                continue;
            }
            for token in split_identifiers(text) {
                *tf.entry(token).or_insert(0.0) += weight;
                len += weight;
            }
        }
        for (term, freq) in tf {
            self.postings.entry(term).or_default().push((id, freq));
        }
        self.doc_len.push(len);
        self.total_len += len;
        id
    }

    /// BM25 score of every document for `query`, indexed by document id.
    pub fn scores(&self, query: &str) -> Vec<f64> {
        let n = self.doc_len.len();
        let mut scores = vec![0.0; n];
        if n == 0 {
            return scores;
        }
        let avg_len = self.total_len / n as f64;
        for term in query_tokens(query) {
            let Some(postings) = self.postings.get(&term) else {
                continue;
            };
            let df = postings.len() as f64;
            let idf = ((n as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            for &(doc, tf) in postings {
                let norm = if avg_len > 0.0 {
                    1.0 - self.b + self.b * self.doc_len[doc] / avg_len
                } else {
                    1.0
                };
                scores[doc] += idf * tf * (self.k1 + 1.0) / (tf + self.k1 * norm);
            }
        }
        scores
    }

    /// Documents with a positive score, best first. Ties are broken by lower id.
    pub fn rank(&self, query: &str) -> Vec<(usize, f64)> {
        let mut ranked: Vec<(usize, f64)> = self
            .scores(query)
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > 0.0)
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked
    }
}
