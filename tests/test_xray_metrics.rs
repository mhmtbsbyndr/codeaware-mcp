use codeaware_mcp::xray::metrics::MetricsState;

#[test]
fn test_new_metrics_state_is_empty() {
    let m = MetricsState::new();
    let snap = m.snapshot();
    assert_eq!(snap.raw_tokens_total, 0);
    assert_eq!(snap.compressed_tokens_total, 0);
    assert_eq!(snap.tool_calls, 0);
    assert!(snap.file_tokens.is_empty());
    assert!(snap.edit_scores.is_empty());
}

#[test]
fn test_record_tool_call_accumulates() {
    let mut m = MetricsState::new();
    m.record_tool_call("smart_read", Some("src/main.rs"), 500, 80);
    m.record_tool_call("smart_read", Some("src/lib.rs"), 300, 50);
    let snap = m.snapshot();
    assert_eq!(snap.tool_calls, 2);
    assert_eq!(snap.raw_tokens_total, 800);
    assert_eq!(snap.compressed_tokens_total, 130);
    assert_eq!(snap.file_tokens.len(), 2);
    assert_eq!(snap.file_tokens["src/main.rs"], 500);
}

#[test]
fn test_record_edit_score() {
    let mut m = MetricsState::new();
    m.record_edit_score("src/auth.rs", "verify_token", 82, "safe");
    let snap = m.snapshot();
    assert_eq!(snap.edit_scores.len(), 1);
    assert_eq!(snap.edit_scores[0].score, 82);
    assert_eq!(snap.edit_scores[0].verdict, "safe");
}

#[test]
fn test_snapshot_serializes_to_json() {
    let mut m = MetricsState::new();
    m.record_tool_call("smart_run", None, 200, 15);
    let snap = m.snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    assert!(json.contains("\"tool_calls\":1"));
    assert!(json.contains("\"raw_tokens_total\":200"));
}

#[test]
fn test_phase_updates() {
    let mut m = MetricsState::new();
    m.set_phase("Analyzing");
    let snap = m.snapshot();
    assert_eq!(snap.phase, "Analyzing");
}
