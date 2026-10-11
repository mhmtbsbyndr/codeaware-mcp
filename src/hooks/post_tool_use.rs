use crate::hooks::spool;
use crate::xray::metrics::MetricsState;
use crate::xray::shared;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

/// Edit-like tools: a file changed, so the cached context must be rebuilt.
pub fn is_file_edit_tool(tool_name: &str) -> bool {
    let lower = tool_name.to_ascii_lowercase();
    lower.contains("edit") || lower == "write" || lower.ends_with("__write")
}

/// Called from CLI hook path where no MetricsState exists.
/// The event is spooled and recorded by the next drain (next hook call or
/// session stop). If the spool is unavailable, metrics are written directly.
pub fn handle_post_tool_use(input: &str) -> Result<String, HookError> {
    let parsed: Value = serde_json::from_str(input)?;
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");
    let output_size = parsed["tool_output_size"].as_u64().unwrap_or(0);
    let file_path = parsed["file_path"].as_str();

    let estimated_tokens = output_size / 4;
    let compressed_estimate = estimated_tokens * 10 / 100;

    let kind = if is_file_edit_tool(tool_name) { "file_edit" } else { "observation" };
    let payload = json!({
        "tool_name": tool_name,
        "file_path": file_path,
        "estimated_tokens": estimated_tokens,
        "compressed_estimate": compressed_estimate,
        "project": parsed["project"].as_str().unwrap_or("."),
    });
    if let Err(e) = spool::enqueue(kind, &payload) {
        eprintln!("CodeAware: spool unavailable ({e}); recording metrics directly");
        shared::append_tool_call(tool_name, file_path, estimated_tokens, compressed_estimate);
    }

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
