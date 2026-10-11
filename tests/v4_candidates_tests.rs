//! Candidate list (`semantic_context::rank_candidates`), RRF over paths and the output mode of
//! `get_task_context`.

use codeaware_mcp::v4::cochange::CoChangeHistory;
use codeaware_mcp::v4::context_items::ContextItemKind;
use codeaware_mcp::v4::contracts::TaskIntent;
use codeaware_mcp::v4::ranking::rrf_paths;
use codeaware_mcp::v4::semantic_context::{
    cap_words, fuse_with_cochange, rank_candidates, rank_file_candidates, FileCandidate, SemanticContextOptions,
    CANDIDATE_REASON_MAX_WORDS,
};
use codeaware_mcp::v4::semantic_index::SemanticIndex;
use codeaware_mcp::v4::symbols::{CodeSymbol, SymbolKind};
use codeaware_mcp::v4::tools::{ContextOutput, GetTaskContextRequest, V4Tools};

fn index_with(symbols: &[(&str, &str)]) -> SemanticIndex {
    let mut index = SemanticIndex::default();
    for (name, path) in symbols {
        index.symbols.symbols.push(CodeSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            path: path.to_string(),
            line_start: 1,
            line_end: 2,
            signature: Some(format!("pub fn {name}()")),
        });
    }
    index
}

fn sample_index() -> SemanticIndex {
    index_with(&[
        ("parse_config", "src/config.rs"),
        ("load_config", "src/config.rs"),
        ("write_config", "src/config.rs"),
        ("run_cli", "src/cli.rs"),
        ("render_table", "src/other.rs"),
    ])
}

#[test]
fn candidates_rank_files_by_matching_symbols_and_skip_unrelated_files() {
    let index = sample_index();
    let list = rank_file_candidates("parse config loader", &index, &SemanticContextOptions::default());
    assert!(!list.is_empty());
    assert_eq!(list[0].path, "src/config.rs", "config file has three matching symbols");
    assert!((list[0].score - 1.0).abs() < 1e-9, "top candidate is scaled to 1.0");
    assert!(list.iter().all(|c| c.path != "src/other.rs"), "no match, no candidate");
}

#[test]
fn each_file_appears_once_with_its_matched_symbols_as_reason() {
    let index = sample_index();
    let list = rank_file_candidates("parse config loader", &index, &SemanticContextOptions::default());
    let config: Vec<&FileCandidate> = list.iter().filter(|c| c.path == "src/config.rs").collect();
    assert_eq!(config.len(), 1, "one row per file");
    assert!(config[0].reason.starts_with("symbols:"));
    assert!(config[0].reason.contains("parse_config"));
    assert!(config[0].reason.contains("load_config"));
}

#[test]
fn reasons_never_exceed_the_word_cap_and_scores_stay_in_range() {
    let index = index_with(&[
        ("alpha_one", "src/a.rs"),
        ("alpha_two", "src/a.rs"),
        ("alpha_three", "src/a.rs"),
        ("alpha_four", "src/a.rs"),
        ("alpha_five", "src/a.rs"),
        ("alpha_six", "src/b.rs"),
    ]);
    let list = rank_candidates("alpha", &index, None, 10);
    assert_eq!(list.len(), 2);
    for candidate in &list {
        assert!(candidate.reason.split_whitespace().count() <= CANDIDATE_REASON_MAX_WORDS);
        assert!(candidate.score > 0.0 && candidate.score <= 1.0);
    }
    // Three names at most per file: "symbols:" plus three names is four words.
    assert!(list[0].reason.matches(", ").count() <= 2);
}

#[test]
fn bm25_only_list_is_truncated_and_has_no_duplicate_paths() {
    let index = sample_index();
    let list = rank_candidates("config cli table", &index, None, 2);
    assert!(list.len() <= 2);
    let mut paths: Vec<&str> = list.iter().map(|c| c.path.as_str()).collect();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), list.len());
}

#[test]
fn unknown_goal_gives_no_candidates() {
    let index = sample_index();
    assert!(rank_candidates("zzqx", &index, None, 10).is_empty());
}

#[test]
fn cap_words_keeps_first_words_only() {
    assert_eq!(cap_words("one two three four", 2), "one two");
    assert_eq!(cap_words("  a   b  ", 8), "a b");
    assert_eq!(cap_words("", 8), "");
}

#[test]
fn rrf_over_paths_rewards_files_in_both_lists() {
    let fused = rrf_paths(
        &[
            vec!["a".to_string(), "b".to_string()],
            vec!["b".to_string(), "c".to_string()],
        ],
        60.0,
    );
    let order: Vec<&str> = fused.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(order, vec!["b", "a", "c"]);
    assert!((fused[0].1 - (1.0 / 62.0 + 1.0 / 61.0)).abs() < 1e-12);
}

#[test]
fn rrf_ignores_duplicate_entries_within_one_list() {
    let fused = rrf_paths(&[vec!["a".to_string(), "a".to_string(), "b".to_string()]], 60.0);
    assert_eq!(fused[0].0, "a");
    assert!((fused[0].1 - 1.0 / 61.0).abs() < 1e-12);
    assert!((fused[1].1 - 1.0 / 62.0).abs() < 1e-12, "b is the second unique path, rank 2");
}

#[test]
fn cochange_fusion_adds_partner_files_and_marks_them() {
    let index = sample_index();
    // config.rs and cli.rs changed together twice; config.rs also changed alone once.
    let log = "\u{1e}c3\n\nsrc/config.rs\nsrc/cli.rs\n\u{1e}c2\n\nsrc/config.rs\nsrc/cli.rs\n\u{1e}c1\n\nsrc/config.rs\n";
    let history = CoChangeHistory::from_log(log);
    // "cli" matches run_cli too, so both files are BM25 hits and both are seeds.
    let bm25 = rank_file_candidates("parse config cli", &index, &SemanticContextOptions::default());
    let fused = fuse_with_cochange(&bm25, &index, &history, 10);

    let config = fused.iter().find(|c| c.path == "src/config.rs").expect("bm25 file kept");
    assert!(config.reason.ends_with("+ co-change"), "reason: {}", config.reason);
    assert!((config.score - 1.0).abs() < 1e-9);
    assert_eq!(fused[0].path, "src/config.rs", "BM25 top file keeps first place on ties");
}

#[test]
fn cochange_partner_outside_bm25_list_is_added_with_seed_reason() {
    let index = sample_index();
    let log = "\u{1e}c2\n\nsrc/config.rs\nsrc/cli.rs\n\u{1e}c1\n\nsrc/config.rs\nsrc/cli.rs\n";
    let history = CoChangeHistory::from_log(log);
    let bm25 = rank_file_candidates("parse config", &index, &SemanticContextOptions::default());
    let fused = fuse_with_cochange(&bm25, &index, &history, 10);

    let cli = fused.iter().find(|c| c.path == "src/cli.rs").expect("co-change partner added");
    assert_eq!(cli.reason, "co-changed with src/config.rs");
    assert!(cli.score > 0.0 && cli.score <= 1.0);
}

#[test]
fn cochange_partner_without_symbols_is_not_offered() {
    let index = sample_index();
    let log = "\u{1e}c2\n\nsrc/config.rs\nREADME.md\n\u{1e}c1\n\nsrc/config.rs\nREADME.md\n";
    let history = CoChangeHistory::from_log(log);
    let list = rank_candidates("parse config", &index, Some(&history), 10);
    assert!(list.iter().all(|c| c.path != "README.md"));
}

#[test]
fn cochange_output_respects_limit() {
    let index = sample_index();
    let log = "\u{1e}c2\n\nsrc/config.rs\nsrc/cli.rs\nsrc/other.rs\n\u{1e}c1\n\nsrc/config.rs\nsrc/cli.rs\n";
    let history = CoChangeHistory::from_log(log);
    let list = rank_candidates("parse config", &index, Some(&history), 1);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].path, "src/config.rs");
}

#[test]
fn get_task_context_candidates_mode_returns_items_without_bodies() {
    let root = std::env::temp_dir().join(format!("codeaware-candidates-{}", std::process::id()));
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("config.rs"),
        "pub fn parse_config(input: &str) -> String {\n    let secret_body_marker = input.to_string();\n    secret_body_marker\n}\n",
    )
    .unwrap();

    let response = V4Tools::get_task_context_with(
        GetTaskContextRequest {
            task_id: None,
            goal: "parse config".to_string(),
            intent: TaskIntent::ImplementFeature,
        },
        root.to_string_lossy().into_owned(),
        ContextOutput::Candidates,
        None,
    );
    let items = &response.context_package.selected_context;
    assert!(!items.is_empty());
    assert!(items.iter().all(|item| matches!(item.kind, ContextItemKind::FileExcerpt)));
    assert_eq!(items[0].path.as_deref(), Some("src/config.rs"));
    assert!(items.iter().all(|item| !item.content.contains("secret_body_marker")), "no bodies");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn default_get_task_context_still_returns_full_output() {
    let root = std::env::temp_dir().join(format!("codeaware-default-full-{}", std::process::id()));
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("config.rs"), "pub fn parse_config() {}\n").unwrap();

    let req = || GetTaskContextRequest {
        task_id: Some("fixed".to_string()),
        goal: "parse config".to_string(),
        intent: TaskIntent::ImplementFeature,
    };
    let default = V4Tools::get_task_context(req(), root.to_string_lossy().into_owned());
    let full = V4Tools::get_task_context_with(req(), root.to_string_lossy().into_owned(), ContextOutput::Full, None);
    assert_eq!(
        serde_json::to_string(&default.context_package.selected_context).unwrap(),
        serde_json::to_string(&full.context_package.selected_context).unwrap(),
        "default output must be the Full output"
    );
    let _ = std::fs::remove_dir_all(&root);
}
