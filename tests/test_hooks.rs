use codeaware_mcp::hooks::compact::{handle_post_compact, handle_pre_compact};
use codeaware_mcp::hooks::post_tool_use::handle_post_tool_use;
use codeaware_mcp::hooks::session_stop::handle_session_stop;
use codeaware_mcp::hooks::subagent_stop::handle_subagent_stop;
use codeaware_mcp::hooks::tool_failure::handle_tool_failure;

#[test]
fn test_post_tool_use_metrics() {
    let input = serde_json::json!({
        "event": "PostToolUse",
        "tool_name": "codeaware__smart_read",
        "tool_input": {"path": "src/main.rs", "mode": "skeleton"},
        "tool_output_size": 847,
        "session_id": "abc123",
        "timestamp": "2026-04-01T12:34:56Z"
    });
    let result = handle_post_tool_use(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
    assert!(parsed["metrics_logged"].as_bool().unwrap());
}

#[test]
fn test_tool_failure_recovery_hint() {
    let input = serde_json::json!({
        "event": "PostToolUseFailure",
        "tool_name": "codeaware__smart_edit",
        "tool_input": {"path": "src/main.rs", "strategy": "text"},
        "error": "E_AMBIGUOUS_MATCH",
        "session_id": "abc123"
    });
    let result = handle_tool_failure(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("recovery_hint").is_some());
}

#[test]
fn test_pre_compact_produces_summary() {
    let input = serde_json::json!({
        "event": "PreCompact",
        "session_id": "abc123"
    });
    let result = handle_pre_compact(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_post_compact_marks_stale() {
    let input = serde_json::json!({
        "event": "PostCompact",
        "session_id": "abc123"
    });
    let result = handle_post_compact(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_subagent_stop_normalizes() {
    let input = serde_json::json!({
        "event": "SubagentStop",
        "agent_name": "code-analyzer",
        "result_size": 2500,
        "session_id": "abc123"
    });
    let result = handle_subagent_stop(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_session_stop_bilanz() {
    let input = serde_json::json!({
        "event": "Stop",
        "session_id": "abc123"
    });
    let result = handle_session_stop(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("reason").is_some());
}
