use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::v4::bm25::Bm25Index;
use crate::v4::context_items::{ContextItem, ContextItemKind};
use crate::v4::pack::{pack, PackCandidate, PackConfig, PackedItem};
use crate::v4::query::normalize_goal;
use crate::v4::ranking::RankingMode;
use crate::v4::semantic_index::SemanticIndex;
use crate::v4::summaries::{declaration_skeleton, read_capped};
use crate::v4::symbols::{contains_any_term, query_terms, CodeSymbol};
use crate::v4::tokens::estimate_tokens;

/// Relevance base for non-symbol items, scaled by 1/(1+rank) within each category.
const IMPORT_WEIGHT: f64 = 0.5;
const CALL_WEIGHT: f64 = 0.4;
const TEST_WEIGHT: f64 = 0.6;
/// BM25 field weights for symbol selection.
const SYMBOL_NAME_WEIGHT: f64 = 3.0;
const SYMBOL_PATH_WEIGHT: f64 = 1.0;
const SYMBOL_SIGNATURE_WEIGHT: f64 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContextOptions {
    pub max_symbols: usize,
    pub max_imports: usize,
    pub max_calls: usize,
    pub max_tests: usize,
    /// Symbol selection strategy. `Legacy` keeps the substring/term matching.
    #[serde(default)]
    pub ranking: RankingMode,
    /// `None` returns every item in its one-line form (legacy). `Some` packs items under the
    /// configured budget and may return full bodies or skeletons.
    #[serde(default)]
    pub pack: Option<PackConfig>,
    /// Root used to read symbol bodies for the `Full` representation. Without it symbols only
    /// have a skeleton (from the signature) and a one-line form.
    #[serde(default)]
    pub source_root: Option<PathBuf>,
    /// Normalizes the goal before matching (German to English vocabulary, stopwords, identifier
    /// split). `false` uses the raw goal, as before the normalizer existed.
    #[serde(default = "default_normalize_goal")]
    pub normalize_goal: bool,
}

fn default_normalize_goal() -> bool {
    true
}

impl Default for SemanticContextOptions {
    fn default() -> Self {
        Self {
            max_symbols: 12,
            max_imports: 12,
            max_calls: 12,
            max_tests: 12,
            // BM25 with identifier splitting: same token budget, roughly 2x recall@5
            // on the repo benchmark (0.168 -> 0.343). See benches/RESULTS.md.
            ranking: RankingMode::Bm25,
            pack: None,
            source_root: None,
            // Off by default: on the repo benchmark normalisation lowered recall@5
            // (German raw 0.146 vs normalised 0.125). Kept as opt-in.
            normalize_goal: false,
        }
    }
}

pub struct SemanticContextAssembler;

/// An item plus the material needed to offer it in other representations.
struct Scored {
    item: ContextItem,
    relevance: f64,
    skeleton: Option<String>,
    full: Option<String>,
}

impl SemanticContextAssembler {
    /// Assembles context items that relate to the goal. Only symbols, imports, calls and
    /// tests whose names or paths contain a goal term are included, so an unrelated goal
    /// yields no items and callers can fall back to file summaries.
    ///
    /// With `options.pack` set, items are packed under the budget and each item carries the
    /// representation the packer chose (`content` and `estimated_tokens` reflect it).
    pub fn assemble(goal: &str, index: &SemanticIndex, options: SemanticContextOptions) -> Vec<ContextItem> {
        let scored = collect(goal, index, &options);
        let Some(config) = &options.pack else {
            return scored.into_iter().map(|entry| entry.item).collect();
        };

        let candidates: Vec<PackCandidate> = scored
            .iter()
            .enumerate()
            .map(|(i, entry)| PackCandidate {
                id: format!("{i:06}"),
                relevance: entry.relevance,
                full: entry.full.clone(),
                skeleton: entry.skeleton.clone(),
                one_line: entry.item.content.clone(),
            })
            .collect();

        let mut packed: Vec<PackedItem> = pack(config, &candidates).items;
        packed.sort_by(|a, b| a.id.cmp(&b.id));
        packed
            .into_iter()
            .filter_map(|picked| {
                let entry = scored.get(picked.id.parse::<usize>().ok()?)?;
                let mut item = entry.item.clone();
                item.content = picked.representation.text().to_string();
                item.estimated_tokens = picked.tokens;
                Some(item)
            })
            .collect()
    }
}

/// Query string for symbol ranking and the term list for the import, call and test filters.
/// With normalization the query is the normalized terms and identifiers; without it, the raw goal.
fn retrieval_query(goal: &str, options: &SemanticContextOptions) -> (String, Vec<String>) {
    if options.normalize_goal {
        let normalized = normalize_goal(goal);
        (normalized.search_query(), normalized.match_terms())
    } else {
        (goal.to_string(), query_terms(goal))
    }
}

fn collect(goal: &str, index: &SemanticIndex, options: &SemanticContextOptions) -> Vec<Scored> {
    let mut scored = Vec::new();
    let (query, terms) = retrieval_query(goal, options);
    if terms.is_empty() && options.ranking == RankingMode::Legacy {
        return scored;
    }

    for (symbol, relevance) in select_symbols(&query, index, options.ranking)
        .into_iter()
        .take(options.max_symbols)
    {
        let content = format!(
            "Symbol: {:?} {} in {}:{}-{}",
            symbol.kind, symbol.name, symbol.path, symbol.line_start, symbol.line_end
        );
        let body = options
            .source_root
            .as_deref()
            .and_then(|root| symbol_body(root, &symbol));
        let skeleton = symbol
            .signature
            .clone()
            .or_else(|| body.as_deref().and_then(|text| text.lines().next()).and_then(declaration_skeleton));
        scored.push(Scored {
            item: ContextItem {
                kind: ContextItemKind::FileExcerpt,
                path: Some(symbol.path.clone()),
                symbol: Some(symbol.name.clone()),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Matched semantic symbol query.".to_string(),
            },
            relevance,
            skeleton,
            full: body,
        });
    }

    let relevant_imports = index
        .imports
        .edges
        .iter()
        .filter(|edge| contains_any_term(&edge.from_path, &terms) || contains_any_term(&edge.import, &terms));
    for (rank, edge) in relevant_imports.take(options.max_imports).enumerate() {
        let content = format!("Import: {} -> {}", edge.from_path, edge.import);
        scored.push(plain_item(
            ContextItem {
                kind: ContextItemKind::RecentChange,
                path: Some(edge.from_path.clone()),
                symbol: None,
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Import relationship from semantic index.".to_string(),
            },
            IMPORT_WEIGHT / (1.0 + rank as f64),
        ));
    }

    let relevant_calls = index.calls.edges.iter().filter(|edge| {
        contains_any_term(&edge.path, &terms)
            || contains_any_term(&edge.from_symbol, &terms)
            || contains_any_term(&edge.to_symbol, &terms)
    });
    for (rank, edge) in relevant_calls.take(options.max_calls).enumerate() {
        let content = format!("Call: {} -> {} in {}", edge.from_symbol, edge.to_symbol, edge.path);
        scored.push(plain_item(
            ContextItem {
                kind: ContextItemKind::FileExcerpt,
                path: Some(edge.path.clone()),
                symbol: Some(edge.from_symbol.clone()),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Call relationship from semantic index.".to_string(),
            },
            CALL_WEIGHT / (1.0 + rank as f64),
        ));
    }

    let relevant_tests = index.tests.references.iter().filter(|test| {
        contains_any_term(&test.test_path, &terms) || contains_any_term(&test.referenced_symbol, &terms)
    });
    for (rank, test) in relevant_tests.take(options.max_tests).enumerate() {
        let content = format!("Test: {} references {}", test.test_path, test.referenced_symbol);
        scored.push(plain_item(
            ContextItem {
                kind: ContextItemKind::TestHint,
                path: Some(test.test_path.clone()),
                symbol: Some(test.referenced_symbol.clone()),
                estimated_tokens: estimate_tokens(&content),
                content,
                reason: "Test relationship from semantic index.".to_string(),
            },
            TEST_WEIGHT / (1.0 + rank as f64),
        ));
    }

    scored
}

fn plain_item(item: ContextItem, relevance: f64) -> Scored {
    Scored {
        item,
        relevance,
        skeleton: None,
        full: None,
    }
}

/// Symbols with a relevance score, best first. Legacy: term-hit order with `1/(1+rank)`.
/// BM25: BM25 score over name, path and signature, normalised to the top hit (1.0).
fn select_symbols(query: &str, index: &SemanticIndex, ranking: RankingMode) -> Vec<(CodeSymbol, f64)> {
    match ranking {
        RankingMode::Legacy => index
            .find_symbols(query)
            .into_iter()
            .enumerate()
            .map(|(rank, symbol)| (symbol, 1.0 / (1.0 + rank as f64)))
            .collect(),
        RankingMode::Bm25 | RankingMode::Bm25Graph => {
            let mut bm25 = Bm25Index::new();
            for symbol in &index.symbols.symbols {
                bm25.add_document(&[
                    (symbol.name.as_str(), SYMBOL_NAME_WEIGHT),
                    (symbol.path.as_str(), SYMBOL_PATH_WEIGHT),
                    (symbol.signature.as_deref().unwrap_or(""), SYMBOL_SIGNATURE_WEIGHT),
                ]);
            }
            // Scores are divided by the top hit so they share the (0, 1] scale of the other
            // item categories; this keeps a score-gap cut across categories meaningful.
            let hits = bm25.rank(query);
            let top = hits.first().map_or(1.0, |(_, score)| *score);
            hits.into_iter()
                .map(|(id, score)| (index.symbols.symbols[id].clone(), score / top))
                .collect()
        }
    }
}

/// Lines `start..=end` of a symbol, read from `root`. `None` when the file cannot be read.
fn symbol_body(root: &Path, symbol: &CodeSymbol) -> Option<String> {
    let content = read_capped(&root.join(&symbol.path))?;
    let lines: Vec<&str> = content.lines().collect();
    let start = symbol.line_start.max(1) - 1;
    let end = symbol.line_end.min(lines.len());
    (start < end).then(|| lines[start..end].join("\n"))
}
