//! Minimal in-memory BM25 index over pre-tokenized documents.

use std::collections::HashMap;
use std::mem::size_of;

const K1: f64 = 1.2;
const B: f64 = 0.75;

/// BM25 postings and document lengths. Documents are token lists; repeating a
/// token raises its term frequency, which is how field weights are applied.
#[derive(Debug, Clone, Default)]
pub struct Bm25 {
    doc_len: Vec<u32>,
    avg_len: f64,
    postings: HashMap<String, Vec<(u32, u32)>>,
}

impl Bm25 {
    /// Builds the index. Document ids are the positions in `docs`.
    pub fn build(docs: &[Vec<String>]) -> Self {
        let mut postings: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        let mut doc_len = Vec::with_capacity(docs.len());
        for (doc_id, tokens) in docs.iter().enumerate() {
            doc_len.push(tokens.len() as u32);
            let mut tf: HashMap<&str, u32> = HashMap::new();
            for token in tokens {
                *tf.entry(token.as_str()).or_insert(0) += 1;
            }
            for (term, count) in tf {
                postings
                    .entry(term.to_string())
                    .or_default()
                    .push((doc_id as u32, count));
            }
        }
        for list in postings.values_mut() {
            list.sort_unstable();
        }
        let total: u64 = doc_len.iter().map(|l| u64::from(*l)).sum();
        let avg_len = if docs.is_empty() {
            1.0
        } else {
            (total as f64 / docs.len() as f64).max(1.0)
        };
        Self {
            doc_len,
            avg_len,
            postings,
        }
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.doc_len.len()
    }

    /// True when no documents are indexed.
    pub fn is_empty(&self) -> bool {
        self.doc_len.is_empty()
    }

    /// BM25 score of every document for `query` (duplicate query terms count once).
    pub fn score_all(&self, query: &[String]) -> Vec<f64> {
        let n = self.doc_len.len() as f64;
        let mut scores = vec![0.0; self.doc_len.len()];
        let mut terms: Vec<&str> = query.iter().map(String::as_str).collect();
        terms.sort_unstable();
        terms.dedup();
        for term in terms {
            let Some(list) = self.postings.get(term) else {
                continue;
            };
            let df = list.len() as f64;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for &(doc, tf) in list {
                let tf = f64::from(tf);
                let dl = f64::from(self.doc_len[doc as usize]);
                let norm = tf + K1 * (1.0 - B + B * dl / self.avg_len);
                scores[doc as usize] += idf * tf * (K1 + 1.0) / norm;
            }
        }
        scores
    }

    /// Rough heap footprint in bytes (postings and lengths), for reporting.
    pub fn approx_bytes(&self) -> usize {
        let postings: usize = self
            .postings
            .iter()
            .map(|(k, v)| k.len() + v.len() * size_of::<(u32, u32)>() + size_of::<Vec<(u32, u32)>>())
            .sum();
        postings + self.doc_len.len() * size_of::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn rare_matching_term_outranks_common_one() {
        let docs = vec![
            doc(&["alpha", "common"]),
            doc(&["beta", "common"]),
            doc(&["gamma", "common"]),
            doc(&["pdf", "common"]),
        ];
        let bm = Bm25::build(&docs);
        let scores = bm.score_all(&doc(&["pdf"]));
        assert!(scores[3] > 0.0);
        assert_eq!(scores[0], 0.0);
        assert_eq!(bm.len(), 4);
    }

    #[test]
    fn duplicate_query_terms_count_once() {
        let docs = vec![doc(&["pdf"]), doc(&["other"])];
        let bm = Bm25::build(&docs);
        let once = bm.score_all(&doc(&["pdf"]));
        let twice = bm.score_all(&doc(&["pdf", "pdf"]));
        assert_eq!(once, twice);
    }
}
