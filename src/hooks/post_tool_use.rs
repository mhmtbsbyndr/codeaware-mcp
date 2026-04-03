use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_post_tool_use(input: &str) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");
    let output_size = parsed["tool_output_size"].as_u64().unwrap_or(0);

    // Estimate token metrics
    let estimated_tokens = output_size / 4; // rough byte-to-token

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": format!("Tool {tool_name}: ~{estimated_tokens} output tokens"),
        "metrics_logged": true
    })
    .to_string())
}
