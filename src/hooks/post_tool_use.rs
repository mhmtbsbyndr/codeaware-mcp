use crate::xray::metrics::MetricsState;
use crate::xray::shared;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

/// Called from CLI hook path where no MetricsState exists.
/// Writes to shared metrics file so xray dashboard can pick it up.
pub fn handle_post_tool_use(input: &str) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");
    let output_size = parsed["tool_output_size"].as_u64().unwrap_or(0);
    let file_path = parsed["file_path"].as_str();

    let estimated_tokens = output_size / 4;
    let compressed_estimate = estimated_tokens * 10 / 100;

    // Write to shared file so xray server can read it
    shared::append_tool_call(tool_name, file_path, estimated_tokens, compressed_estimate);

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": format!("Tool {tool_name}: ~{estimated_tokens} output tokens"),
        "metrics_logged": true
    })
    .to_string())
}

/// Called from the MCP server path where MetricsState is available.
pub fn handle_post_tool_use_with_metrics(
    input: &str,
    metrics: Option<&Arc<Mutex<MetricsState>>>,
) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");
    let output_size = parsed["tool_output_size"].as_u64().unwrap_or(0);
    let file_path = parsed["file_path"].as_str();

    let estimated_tokens = output_size / 4;
    let compressed_estimate = estimated_tokens * 10 / 100;

    if let Some(m) = metrics {
        if let Ok(mut state) = m.lock() {
            state.record_tool_call(tool_name, file_path, estimated_tokens, compressed_estimate);
        }
    }

    // Only write to shared file if no in-memory MetricsState (CLI hook path)
    // Avoids double-counting: server.rs records in-memory, merge_shared adds file-based
    if metrics.is_none() {
        shared::append_tool_call(tool_name, file_path, estimated_tokens, compressed_estimate);
    }

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": format!("Tool {tool_name}: ~{estimated_tokens} output tokens"),
        "metrics_logged": true
    })
    .to_string())
}
