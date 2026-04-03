use codeaware_mcp::hooks::post_tool_use::handle_post_tool_use_with_metrics;
use codeaware_mcp::xray::metrics::MetricsState;
use std::sync::{Arc, Mutex};

#[test]
fn test_hook_updates_metrics() {
    let metrics = Arc::new(Mutex::new(MetricsState::new()));
    let input = r#"{"tool_name":"smart_read","tool_output_size":400,"file_path":"src/main.rs"}"#;
    let result = handle_post_tool_use_with_metrics(input, Some(&metrics)).unwrap();
    assert!(result.contains("approve"));

    let snap = metrics.lock().unwrap().snapshot();
    assert_eq!(snap.tool_calls, 1);
    assert_eq!(snap.raw_tokens_total, 100); // 400 bytes / 4
    assert!(snap.file_tokens.contains_key("src/main.rs"));
}

#[test]
fn test_hook_without_metrics_still_works() {
    let input = r#"{"tool_name":"smart_run","tool_output_size":800}"#;
    let result = handle_post_tool_use_with_metrics(input, None).unwrap();
    assert!(result.contains("approve"));
}
