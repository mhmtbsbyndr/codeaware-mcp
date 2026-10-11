use std::collections::BTreeSet;

use codeaware_mcp::v4::import_graph::resolve_rust_import;
use codeaware_mcp::v4::ranking::{personalized_pagerank, reciprocal_rank_fusion};
use codeaware_mcp::v4::{ContextRanker, RankSignals, RankingMode, SemanticIndex};

fn known(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

#[test]
fn resolves_crate_use_paths_to_files_and_modules() {
    let files = known(&["src/v4/bm25.rs", "src/v4/mod.rs", "src/v4/pack/item.rs"]);
    assert_eq!(
        resolve_rust_import("src/v4/ranking.rs", "use crate::v4::bm25::Bm25Index", &files).as_deref(),
        Some("src/v4/bm25.rs")
    );
    assert_eq!(
        resolve_rust_import("src/v4/ranking.rs", "pub(crate) use crate::v4::{bm25, pack}", &files).as_deref(),
        Some("src/v4/mod.rs")
    );
    assert_eq!(
        resolve_rust_import("src/v4/pack.rs", "use crate::v4::pack::item::Thing", &files).as_deref(),
        Some("src/v4/pack/item.rs")
    );
}

#[test]
fn resolves_mod_declarations_relative_to_the_importing_file() {
    let files = known(&["src/v4/ranking/tests.rs", "src/v4/child.rs"]);
    assert_eq!(
        resolve_rust_import("src/v4/ranking.rs", "mod tests;", &files).as_deref(),
        Some("src/v4/ranking/tests.rs")
    );
    assert_eq!(
        resolve_rust_import("src/v4/mod.rs", "pub mod child;", &files).as_deref(),
        Some("src/v4/child.rs")
    );
}

#[test]
fn external_and_unknown_imports_do_not_resolve() {
    let files = known(&["src/v4/bm25.rs"]);
    assert_eq!(resolve_rust_import("src/v4/x.rs", "use std::collections::BTreeMap", &files), None);
    assert_eq!(resolve_rust_import("src/v4/x.rs", "use serde::Serialize", &files), None);
    assert_eq!(resolve_rust_import("src/v4/x.rs", "use crate::v4::missing", &files), None);
}

#[test]
fn personalized_pagerank_keeps_mass_and_favours_near_seeds() {
    // Chain 0 - 1 - 2 (both directions), seed on node 0.
    let edges = vec![(0, 1), (1, 0), (1, 2), (2, 1)];
    let seeds = vec![1.0, 0.0, 0.0];
    let rank = personalized_pagerank(3, &edges, &seeds, 0.85, 30);

    let total: f64 = rank.iter().sum();
    assert!((total - 1.0).abs() < 1e-9, "mass {total}");
    // Node 1 is a hub on the chain, so it can out-rank the seed; the far node must rank lowest.
    assert!(rank[2] < rank[0] && rank[2] < rank[1], "{rank:?}");

    let again = personalized_pagerank(3, &edges, &seeds, 0.85, 30);
    assert_eq!(rank, again);
}

#[test]
fn reciprocal_rank_fusion_rewards_agreement() {
    let fused = reciprocal_rank_fusion(3, &[vec![0, 1], vec![1, 2]], 60.0);
    assert!(fused[1] > fused[0] && fused[1] > fused[2]);
    assert!((fused[0] - 1.0 / 61.0).abs() < 1e-12);
}

#[test]
fn graph_mode_pulls_in_imported_file_without_keyword_hit() {
    let mut index = SemanticIndex::default();
    index.imports.add("src/invoice.rs", "use crate::z_helper::run");

    let paths = vec![
        "src/invoice.rs".to_string(),
        "src/m_other.rs".to_string(),
        "src/z_helper.rs".to_string(),
    ];
    let signals = RankSignals {
        index: Some(&index),
        doc_heads: None,
    };

    let bm25 = ContextRanker::rank(RankingMode::Bm25, "invoice", paths.clone(), signals);
    let graph = ContextRanker::rank(RankingMode::Bm25Graph, "invoice", paths, signals);

    assert_eq!(bm25[0].path, "src/invoice.rs");
    assert_eq!(bm25.last().map(|i| i.path.as_str()), Some("src/z_helper.rs"));

    assert_eq!(graph[0].path, "src/invoice.rs");
    assert_eq!(graph[1].path, "src/z_helper.rs", "graph neighbour ranks above unconnected file");
    assert_eq!(graph[1].reason, "Reached via import or call graph");
    assert_eq!(graph[2].path, "src/m_other.rs");
}

#[test]
fn graph_mode_without_seeds_falls_back_to_path_order() {
    let mut index = SemanticIndex::default();
    index.imports.add("src/a.rs", "use crate::b::x");
    let paths = vec!["src/b.rs".to_string(), "src/a.rs".to_string()];
    let signals = RankSignals {
        index: Some(&index),
        doc_heads: None,
    };
    let ranked = ContextRanker::rank(RankingMode::Bm25Graph, "zebra", paths, signals);
    assert_eq!(ranked.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["src/a.rs", "src/b.rs"]);
    assert!(ranked.iter().all(|i| i.relevance == 0.0));
}
