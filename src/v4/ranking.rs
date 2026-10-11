use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::v4::bm25::Bm25Index;
use crate::v4::query::normalize_goal;
use crate::v4::semantic_index::SemanticIndex;

/// Ranking strategy. `Legacy` is the substring counter used before v4 BM25 and stays the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankingMode {
    /// Substring count of goal terms in the lowercased path.
    #[default]
    Legacy,
    /// BM25 over path, symbol names and first doc lines, with identifier splitting.
    Bm25,
    /// BM25 hits used as seeds of a personalised PageRank over the import and call graph,
    /// fused with the BM25 ranking by reciprocal rank fusion.
    Bm25Graph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedPath {
    pub path: String,
    /// Integer score for compatibility. Legacy: the term count. BM25 modes: `relevance * 1000`.
    pub score: usize,
    pub reason: String,
    /// Strategy-specific relevance. Equals `score` in Legacy mode.
    #[serde(default)]
    pub relevance: f64,
}

/// Optional side information for the non-legacy modes.
#[derive(Debug, Clone, Copy, Default)]
pub struct RankSignals<'a> {
    /// Semantic index for symbol names and the import and call graph.
    pub index: Option<&'a SemanticIndex>,
    /// First doc lines per path, as produced by `summaries::doc_head`.
    pub doc_heads: Option<&'a HashMap<String, String>>,
}

pub const PATH_WEIGHT: f64 = 2.0;
pub const SYMBOL_WEIGHT: f64 = 1.5;
pub const HEAD_WEIGHT: f64 = 1.0;
pub const RRF_K: f64 = 60.0;
pub const PPR_DAMPING: f64 = 0.85;
pub const PPR_ITERATIONS: usize = 30;
/// Symbol names defined in more files than this are ignored for call edges. Names such as
/// `new` or `from` would otherwise connect unrelated files.
pub const MAX_DEFINITION_FANOUT: usize = 8;

pub struct ContextRanker;

impl ContextRanker {
    /// Legacy substring ranking. Kept unchanged so the benchmark can compare against it.
    pub fn rank_paths(goal: &str, paths: Vec<String>) -> Vec<RankedPath> {
        let goal_terms: Vec<String> = goal
            .to_lowercase()
            .split_whitespace()
            .map(|term| term.to_string())
            .collect();

        let mut ranked: Vec<RankedPath> = paths
            .into_iter()
            .map(|path| {
                let lower = path.to_lowercase();

                let score = goal_terms
                    .iter()
                    .filter(|term| term.len() > 2)
                    .filter(|term| lower.contains(term.as_str()))
                    .count();

                RankedPath {
                    path,
                    score,
                    reason: if score > 0 {
                        "Matched goal keywords".to_string()
                    } else {
                        "Fallback candidate".to_string()
                    },
                    relevance: score as f64,
                }
            })
            .collect();

        ranked.sort_by_key(|item| std::cmp::Reverse(item.score));
        ranked
    }

    /// Ranks `paths` with the given strategy. Ties are broken by path, so output is stable.
    /// The BM25 modes rank by the normalized goal (see `query::normalize_goal`); `Legacy` keeps
    /// the raw goal so it stays a fixed reference for the benchmark.
    pub fn rank(mode: RankingMode, goal: &str, paths: Vec<String>, signals: RankSignals<'_>) -> Vec<RankedPath> {
        match mode {
            RankingMode::Legacy => Self::rank_paths(goal, paths),
            RankingMode::Bm25 => {
                let query = normalize_goal(goal).search_query();
                let relevance = build_bm25(&paths, &signals).scores(&query);
                let reasons = relevance
                    .iter()
                    .map(|score| if *score > 0.0 { "Matched goal keywords" } else { "Fallback candidate" })
                    .collect();
                finalize(paths, relevance, reasons)
            }
            RankingMode::Bm25Graph => rank_with_graph(&normalize_goal(goal).search_query(), paths, &signals),
        }
    }
}

/// BM25 index over paths (weight `PATH_WEIGHT`), symbol names of each file (`SYMBOL_WEIGHT`)
/// and first doc lines (`HEAD_WEIGHT`). Document id = position in `paths`.
fn build_bm25(paths: &[String], signals: &RankSignals<'_>) -> Bm25Index {
    let mut symbols_by_path: HashMap<&str, Vec<&str>> = HashMap::new();
    if let Some(index) = signals.index {
        for symbol in &index.symbols.symbols {
            symbols_by_path
                .entry(symbol.path.as_str())
                .or_default()
                .push(symbol.name.as_str());
        }
    }

    let mut bm25 = Bm25Index::new();
    for path in paths {
        let symbols = symbols_by_path
            .get(path.as_str())
            .map(|names| names.join(" "))
            .unwrap_or_default();
        let head = signals
            .doc_heads
            .and_then(|heads| heads.get(path))
            .map(String::as_str)
            .unwrap_or("");
        bm25.add_document(&[
            (path.as_str(), PATH_WEIGHT),
            (symbols.as_str(), SYMBOL_WEIGHT),
            (head, HEAD_WEIGHT),
        ]);
    }
    bm25
}

/// BM25 seeds, graph propagation, and reciprocal rank fusion.
fn rank_with_graph(goal: &str, paths: Vec<String>, signals: &RankSignals<'_>) -> Vec<RankedPath> {
    let bm25_scores = build_bm25(&paths, signals).scores(goal);
    let total: f64 = bm25_scores.iter().sum();
    if total <= 0.0 {
        // No seeds: the graph has nothing to personalise on.
        let reasons = vec!["Fallback candidate"; paths.len()];
        return finalize(paths, bm25_scores, reasons);
    }

    let seeds: Vec<f64> = bm25_scores.iter().map(|score| score / total).collect();
    let edges = file_graph_edges(&paths, signals.index);
    let propagated = personalized_pagerank(paths.len(), &edges, &seeds, PPR_DAMPING, PPR_ITERATIONS);

    let bm25_order = order_positive(&bm25_scores, 0.0);
    let graph_order = order_positive(&propagated, 1e-12);
    let fused = reciprocal_rank_fusion(paths.len(), &[bm25_order, graph_order], RRF_K);

    let reasons: Vec<&'static str> = (0..paths.len())
        .map(|i| {
            if bm25_scores[i] > 0.0 {
                "Matched goal keywords"
            } else if propagated[i] > 1e-12 {
                "Reached via import or call graph"
            } else {
                "Fallback candidate"
            }
        })
        .collect();
    finalize(paths, fused, reasons)
}

/// Undirected file-level edges from imports and resolved calls, as index pairs.
fn file_graph_edges(paths: &[String], index: Option<&SemanticIndex>) -> Vec<(usize, usize)> {
    let Some(index) = index else {
        return Vec::new();
    };
    let known: BTreeSet<String> = paths.iter().cloned().collect();
    let position: BTreeMap<&str, usize> = paths.iter().enumerate().map(|(i, p)| (p.as_str(), i)).collect();

    let mut defs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for symbol in &index.symbols.symbols {
        if known.contains(&symbol.path) {
            let files = defs.entry(symbol.name.clone()).or_default();
            if !files.contains(&symbol.path) {
                files.push(symbol.path.clone());
            }
        }
    }
    defs.retain(|_, files| files.len() <= MAX_DEFINITION_FANOUT);

    let mut pairs: Vec<(String, String)> = index.imports.file_edges(&known);
    pairs.extend(index.calls.file_edges(&defs));

    let mut undirected = BTreeSet::new();
    for (a, b) in pairs {
        if let (Some(&ia), Some(&ib)) = (position.get(a.as_str()), position.get(b.as_str())) {
            if ia != ib {
                undirected.insert((ia.min(ib), ia.max(ib)));
            }
        }
    }
    let mut edges = Vec::with_capacity(undirected.len() * 2);
    for (a, b) in undirected {
        edges.push((a, b));
        edges.push((b, a));
    }
    edges
}

/// Personalised PageRank by power iteration with a fixed iteration count. `seeds` must sum to
/// one. Dangling mass is returned to the seeds, so the result keeps total mass one.
pub fn personalized_pagerank(
    n: usize,
    edges: &[(usize, usize)],
    seeds: &[f64],
    damping: f64,
    iterations: usize,
) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mut out_degree = vec![0usize; n];
    for &(from, _) in edges {
        out_degree[from] += 1;
    }

    let mut rank = seeds.to_vec();
    for _ in 0..iterations {
        let mut next: Vec<f64> = seeds.iter().map(|seed| (1.0 - damping) * seed).collect();
        let mut dangling = 0.0;
        for (node, &degree) in out_degree.iter().enumerate() {
            if degree == 0 {
                dangling += rank[node];
            }
        }
        for &(from, to) in edges {
            next[to] += damping * rank[from] / out_degree[from] as f64;
        }
        for (slot, seed) in next.iter_mut().zip(seeds) {
            *slot += damping * dangling * seed;
        }
        rank = next;
    }
    rank
}

/// Reciprocal rank fusion: each list adds `1 / (k + rank)` (rank is 1-based) to its members.
pub fn reciprocal_rank_fusion(n: usize, rankings: &[Vec<usize>], k: f64) -> Vec<f64> {
    let mut fused = vec![0.0; n];
    for ranking in rankings {
        for (position, &doc) in ranking.iter().enumerate() {
            fused[doc] += 1.0 / (k + position as f64 + 1.0);
        }
    }
    fused
}

/// Reciprocal rank fusion over path lists (best first). Returns each path with its fused score,
/// best first, ties by path. Paths missing from a list receive nothing from it.
pub fn rrf_paths(rankings: &[Vec<String>], k: f64) -> Vec<(String, f64)> {
    let mut fused: BTreeMap<&str, f64> = BTreeMap::new();
    for ranking in rankings {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for path in ranking {
            // Rank is 1-based over first occurrences, so duplicates do not push others down.
            if seen.insert(path.as_str()) {
                *fused.entry(path.as_str()).or_insert(0.0) += 1.0 / (k + seen.len() as f64);
            }
        }
    }
    let mut out: Vec<(String, f64)> = fused.into_iter().map(|(p, s)| (p.to_string(), s)).collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out
}

/// Indices with a score above `threshold`, best first, ties by lower index.
fn order_positive(scores: &[f64], threshold: f64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).filter(|&i| scores[i] > threshold).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]).then(a.cmp(&b)));
    order
}

/// Builds the output list: sorted by relevance (desc), then path (asc). `reasons[i]` belongs to
/// `paths[i]`.
fn finalize(paths: Vec<String>, relevance: Vec<f64>, reasons: Vec<&'static str>) -> Vec<RankedPath> {
    let mut items: Vec<RankedPath> = paths
        .into_iter()
        .zip(relevance)
        .zip(reasons)
        .map(|((path, relevance), reason)| RankedPath {
            path,
            score: (relevance * 1000.0).round().max(0.0) as usize,
            reason: reason.to_string(),
            relevance,
        })
        .collect();
    items.sort_by(|a, b| b.relevance.total_cmp(&a.relevance).then_with(|| a.path.cmp(&b.path)));
    items
}
