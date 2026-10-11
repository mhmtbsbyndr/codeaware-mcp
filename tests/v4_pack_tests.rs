use codeaware_mcp::v4::pack::{pack, score_gap_cutoff, PackCandidate, PackConfig, PackMode, Representation};
use codeaware_mcp::v4::{
    declaration_skeleton, doc_head, CodeSymbol, ContextItemKind, SemanticContextAssembler,
    SemanticContextOptions, SemanticIndex, SummaryGenerator, SymbolKind, RankingMode,
};

/// Text of exactly `tokens * 4` letters, so `estimate_tokens` returns `tokens`.
fn letters(tokens: usize) -> String {
    "a".repeat(tokens * 4)
}

fn candidate(id: &str, relevance: f64, full_tokens: Option<usize>, one_line_tokens: usize) -> PackCandidate {
    PackCandidate {
        id: id.to_string(),
        relevance,
        full: full_tokens.map(letters),
        skeleton: None,
        one_line: letters(one_line_tokens),
    }
}

fn config(mode: PackMode, budget: usize) -> PackConfig {
    PackConfig {
        mode,
        budget_tokens: budget,
        early_stop_fraction: None,
    }
}

#[test]
fn score_gap_cutoff_keeps_items_above_fraction_of_top() {
    assert_eq!(score_gap_cutoff(&[10.0, 9.0, 8.0, 2.0, 1.0], 0.3), 3);
    assert_eq!(score_gap_cutoff(&[10.0, 9.0, 8.0, 2.0, 1.0], 0.95), 1);
    assert_eq!(score_gap_cutoff(&[], 0.3), 0);
    assert_eq!(score_gap_cutoff(&[0.0, 0.0], 0.3), 0);
}

#[test]
fn early_stop_lets_easy_task_return_very_little() {
    let candidates = vec![
        candidate("a", 1.0, Some(10), 2),
        candidate("b", 0.1, Some(10), 2),
        candidate("c", 0.05, Some(10), 2),
    ];
    let mut cfg = config(PackMode::Knapsack, 10_000);
    cfg.early_stop_fraction = Some(0.3);
    let result = pack(&cfg, &candidates);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, "a");
    assert_eq!(result.cut_by_score_gap, 2);
}

#[test]
fn legacy_mode_fills_prefix_and_stops_at_first_misfit() {
    let candidates = vec![
        candidate("a", 1.0, Some(40), 5),
        candidate("b", 0.9, Some(100), 5),
        candidate("c", 0.8, Some(10), 5),
    ];
    let result = pack(&config(PackMode::Legacy, 60), &candidates);
    let ids: Vec<&str> = result.items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, vec!["a"], "c would fit but legacy stops at b");
    assert!(result.total_tokens <= 60);
}

#[test]
fn knapsack_offers_one_line_summaries_where_legacy_returns_nothing() {
    let candidates: Vec<PackCandidate> = (0..5)
        .map(|i| candidate(&format!("f{i}"), 1.0 - i as f64 * 0.01, Some(400), 10))
        .collect();
    let legacy = pack(&config(PackMode::Legacy, 60), &candidates);
    let knapsack = pack(&config(PackMode::Knapsack, 60), &candidates);

    assert!(legacy.items.is_empty());
    assert_eq!(knapsack.items.len(), 5);
    assert!(knapsack
        .items
        .iter()
        .all(|item| matches!(item.representation, Representation::OneLine(_))));
    assert!(knapsack.total_tokens <= 60);
}

#[test]
fn knapsack_upgrades_to_full_body_when_extra_cost_fits() {
    let candidates = vec![candidate("only", 1.0, Some(20), 5)];
    let result = pack(&config(PackMode::Knapsack, 25), &candidates);
    assert_eq!(result.items.len(), 1);
    assert!(matches!(result.items[0].representation, Representation::Full(_)));
    assert_eq!(result.total_tokens, 20);
}

#[test]
fn knapsack_final_check_prefers_single_high_value_item_over_greedy_density() {
    // S is denser (0.15 / 10 tokens) and blocks T (0.3 / 100 tokens) under budget 100.
    // The best single option that fits is T, so the final check returns only T.
    let candidates = vec![
        PackCandidate {
            id: "s".to_string(),
            relevance: 0.5,
            full: None,
            skeleton: None,
            one_line: letters(10),
        },
        PackCandidate {
            id: "t".to_string(),
            relevance: 1.0,
            full: None,
            skeleton: None,
            one_line: letters(100),
        },
    ];
    let result = pack(&config(PackMode::Knapsack, 100), &candidates);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, "t");
    assert!((result.total_value - 0.3).abs() < 1e-9);
}

#[test]
fn knapsack_is_deterministic_and_order_independent() {
    let mut candidates: Vec<PackCandidate> = (0..8)
        .map(|i| candidate(&format!("c{i}"), 1.0 - i as f64 * 0.05, Some(30 + i * 7), 4))
        .collect();
    let forward = pack(&config(PackMode::Knapsack, 120), &candidates);
    candidates.reverse();
    let backward = pack(&config(PackMode::Knapsack, 120), &candidates);

    let ids = |r: &codeaware_mcp::v4::PackResult| {
        r.items.iter().map(|item| (item.id.clone(), item.tokens)).collect::<Vec<_>>()
    };
    assert_eq!(ids(&forward), ids(&backward));
    assert!(forward.total_tokens <= 120);
}

#[test]
fn non_positive_relevance_is_never_packed() {
    let candidates = vec![
        candidate("zero", 0.0, Some(1), 1),
        candidate("neg", -1.0, Some(1), 1),
        candidate("nan", f64::NAN, Some(1), 1),
    ];
    let result = pack(&config(PackMode::Knapsack, 1000), &candidates);
    assert!(result.items.is_empty());
}

#[test]
fn skeleton_representation_is_used_when_it_is_the_best_fit() {
    let candidates = vec![PackCandidate {
        id: "api".to_string(),
        relevance: 1.0,
        full: Some(letters(500)),
        skeleton: Some(letters(30)),
        one_line: letters(5),
    }];
    let result = pack(&config(PackMode::Knapsack, 40), &candidates);
    assert_eq!(result.items.len(), 1);
    assert!(matches!(result.items[0].representation, Representation::Skeleton(_)));
}

#[test]
fn declaration_skeleton_keeps_signatures_and_drops_bodies() {
    assert_eq!(
        declaration_skeleton("    pub fn rank(mode: RankingMode, goal: &str) -> Vec<RankedPath> {").as_deref(),
        Some("pub fn rank(mode: RankingMode, goal: &str) -> Vec<RankedPath> { ... }")
    );
    assert_eq!(
        declaration_skeleton("pub struct Foo;").as_deref(),
        Some("pub struct Foo;")
    );
    assert_eq!(declaration_skeleton("let x = 1;"), None);
}

#[test]
fn summary_skeleton_and_one_line_are_compact() {
    let source = "//! Ranking helpers.\n\nuse std::fmt;\n\npub fn alpha() -> u8 {\n    1\n}\n\nstruct Beta;\n";
    let skeleton = SummaryGenerator::skeleton("src/x.rs", source);
    assert!(skeleton.contains("pub fn alpha() -> u8 { ... }"));
    assert!(skeleton.contains("struct Beta;"));
    assert!(!skeleton.contains("use std::fmt"));

    let one_line = SummaryGenerator::one_line("src/x.rs", source);
    assert_eq!(one_line, "src/x.rs (9 lines): Ranking helpers.");
    assert!(!one_line.contains('\n'));

    assert_eq!(doc_head("fn a() {}\n", 2), "");
    assert_eq!(SummaryGenerator::skeleton("e.rs", ""), "e.rs: no declarations");
}

fn symbol_index() -> SemanticIndex {
    let mut index = SemanticIndex::default();
    index.symbols.add(CodeSymbol {
        name: "build_context_package".to_string(),
        kind: SymbolKind::Function,
        path: "src/v4/context.rs".to_string(),
        line_start: 1,
        line_end: 3,
        signature: Some("pub fn build_context_package() -> Package".to_string()),
    });
    index.symbols.add(CodeSymbol {
        name: "context_helper".to_string(),
        kind: SymbolKind::Function,
        path: "src/v4/other.rs".to_string(),
        line_start: 1,
        line_end: 1,
        signature: None,
    });
    index
}

#[test]
fn bm25_symbol_selection_puts_full_term_match_first() {
    let index = symbol_index();
    let options = SemanticContextOptions {
        ranking: RankingMode::Bm25,
        ..Default::default()
    };
    let items = SemanticContextAssembler::assemble("context package", &index, options);
    assert_eq!(items[0].symbol.as_deref(), Some("build_context_package"));
}

#[test]
fn default_semantic_options_keep_one_line_items() {
    let index = symbol_index();
    let items = SemanticContextAssembler::assemble("context", &index, SemanticContextOptions::default());
    assert!(items.iter().all(|item| item.content.starts_with("Symbol:")));
    assert!(items.iter().any(|item| matches!(item.kind, ContextItemKind::FileExcerpt)));
}

#[test]
fn packed_semantic_context_uses_skeletons_and_respects_budget() {
    let index = symbol_index();
    let options = SemanticContextOptions {
        ranking: RankingMode::Bm25,
        pack: Some(config(PackMode::Knapsack, 12)),
        ..Default::default()
    };
    let items = SemanticContextAssembler::assemble("context package", &index, options);
    let total: usize = items.iter().map(|item| item.estimated_tokens).sum();
    assert!(total <= 12, "total {total}");
    assert!(items.iter().any(|item| item.content.starts_with("pub fn build_context_package")));
}

#[test]
fn packed_semantic_context_returns_full_body_when_source_is_available() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src/v4")).unwrap();
    std::fs::write(
        dir.path().join("src/v4/context.rs"),
        "pub fn build_context_package() -> Package {\n    Package::new()\n}\n",
    )
    .unwrap();

    let index = symbol_index();
    let options = SemanticContextOptions {
        ranking: RankingMode::Bm25,
        pack: Some(config(PackMode::Knapsack, 10_000)),
        source_root: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let items = SemanticContextAssembler::assemble("context package", &index, options);
    let body = items
        .iter()
        .find(|item| item.symbol.as_deref() == Some("build_context_package"))
        .expect("symbol item");
    assert!(body.content.contains("Package::new()"), "{}", body.content);
}
