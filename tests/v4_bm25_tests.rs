use codeaware_mcp::v4::bm25::{query_tokens, Bm25Index};
use codeaware_mcp::v4::{split_identifiers, ContextRanker, RankSignals, RankingMode};

#[test]
fn splits_camel_snake_acronym_and_digits() {
    assert_eq!(split_identifiers("buildContextPackage"), vec!["build", "context", "package"]);
    assert_eq!(split_identifiers("HTTPServer"), vec!["http", "server"]);
    assert_eq!(split_identifiers("snake_case_name"), vec!["snake", "case", "name"]);
    assert_eq!(split_identifiers("bm25ranker"), vec!["bm", "25", "ranker"]);
    assert_eq!(split_identifiers("src/v4/bm25.rs"), vec!["src", "v", "4", "bm", "25", "rs"]);
    assert!(split_identifiers("").is_empty());
}

#[test]
fn query_tokens_drop_stop_words_short_terms_and_duplicates() {
    assert_eq!(
        query_tokens("fix the BM25 ranking for paths, ranking"),
        vec!["bm", "25", "ranking", "paths"]
    );
}

#[test]
fn rare_query_term_outranks_common_term() {
    let mut index = Bm25Index::new();
    index.add_document(&[("alpha common words", 1.0)]);
    index.add_document(&[("beta common words", 1.0)]);
    index.add_document(&[("gamma common words", 1.0)]);
    index.add_document(&[("rare_token common words", 1.0)]);

    let ranked = index.rank("rare token");
    assert_eq!(ranked.first().map(|hit| hit.0), Some(3));
    assert_eq!(ranked.len(), 1, "only the rare document matches");
}

#[test]
fn heavier_field_wins_over_lighter_field() {
    let mut index = Bm25Index::new();
    index.add_document(&[("unrelated", 2.0), ("context", 1.0)]);
    index.add_document(&[("context", 2.0), ("unrelated", 1.0)]);

    let ranked = index.rank("context");
    assert_eq!(ranked[0].0, 1);
}

#[test]
fn ties_break_by_lower_document_id_and_empty_index_is_empty() {
    let empty = Bm25Index::new();
    assert!(empty.rank("anything").is_empty());

    let mut index = Bm25Index::new();
    index.add_document(&[("context package", 1.0)]);
    index.add_document(&[("context package", 1.0)]);
    let ranked = index.rank("context package");
    assert_eq!(ranked.iter().map(|hit| hit.0).collect::<Vec<_>>(), vec![0, 1]);
}

#[test]
fn bm25_ranking_prefers_full_term_match_in_path() {
    let paths = vec![
        "README.md".to_string(),
        "src/v4/pack.rs".to_string(),
        "src/v4/context.rs".to_string(),
    ];
    let ranked = ContextRanker::rank(RankingMode::Bm25, "context package", paths, RankSignals::default());
    assert_eq!(ranked[0].path, "src/v4/context.rs");
    assert!(ranked[0].relevance > 0.0);
    assert!(ranked[1..].iter().all(|item| item.relevance == 0.0));
    assert_eq!(ranked[1].path, "README.md", "zero-relevance paths sort by path");
}

#[test]
fn bm25_ranking_is_deterministic_and_uses_symbols_and_doc_heads() {
    use codeaware_mcp::v4::{CodeSymbol, SemanticIndex, SymbolKind};
    use std::collections::HashMap;

    let mut index = SemanticIndex::default();
    index.symbols.add(CodeSymbol {
        name: "invoice_total".to_string(),
        kind: SymbolKind::Function,
        path: "src/billing.rs".to_string(),
        line_start: 1,
        line_end: 2,
        signature: None,
    });
    let mut heads = HashMap::new();
    heads.insert("src/notes.rs".to_string(), "Invoice rounding notes".to_string());

    let paths = vec![
        "src/aaa.rs".to_string(),
        "src/billing.rs".to_string(),
        "src/notes.rs".to_string(),
    ];
    let signals = RankSignals {
        index: Some(&index),
        doc_heads: Some(&heads),
    };
    let first = ContextRanker::rank(RankingMode::Bm25, "invoice", paths.clone(), signals);
    let second = ContextRanker::rank(RankingMode::Bm25, "invoice", paths, signals);

    let order = |items: &[codeaware_mcp::v4::RankedPath]| items.iter().map(|i| i.path.clone()).collect::<Vec<_>>();
    assert_eq!(order(&first), order(&second));
    assert_eq!(first[0].path, "src/billing.rs", "symbol hit");
    assert_eq!(first[1].path, "src/notes.rs", "doc head hit");
    assert_eq!(first[2].path, "src/aaa.rs", "no hit, sorted by path");
    assert_eq!(first[2].reason, "Fallback candidate");
}

#[test]
fn legacy_ranking_is_unchanged_by_default_mode() {
    let paths = vec!["src/v4/context.rs".to_string(), "README.md".to_string()];
    let legacy = ContextRanker::rank_paths("context", paths.clone());
    let via_mode = ContextRanker::rank(RankingMode::Legacy, "context", paths, RankSignals::default());
    assert_eq!(legacy[0].path, via_mode[0].path);
    assert_eq!(legacy[0].score, 1);
    assert_eq!(legacy[0].relevance, 1.0);
}
