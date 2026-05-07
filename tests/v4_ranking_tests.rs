use codeaware_mcp::v4::{estimate_tokens, ContextRanker};

#[test]
fn token_estimation_returns_non_zero_for_text() {
    let tokens = estimate_tokens("CodeAware v4 context package");
    assert!(tokens > 0);
}

#[test]
fn ranking_prefers_matching_paths() {
    let ranked = ContextRanker::rank_paths(
        "memory context",
        vec![
            "src/v4/context.rs".to_string(),
            "src/v4/memory.rs".to_string(),
            "README.md".to_string(),
        ],
    );

    assert!(!ranked.is_empty());
    assert!(ranked[0].score >= ranked[1].score);
}
