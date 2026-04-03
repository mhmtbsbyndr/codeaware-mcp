use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_subagent_stop(input: &str) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let result_size = parsed["result_size"].as_u64().unwrap_or(0);
    let agent = parsed["agent_name"].as_str().unwrap_or("unknown");

    let warning = if result_size > 5000 {
        Some(format!(
            "Agent {agent} result is large ({result_size} bytes). Consider compression."
        ))
    } else {
        None
    };

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": format!("Agent {agent} completed, result: {result_size} bytes"),
        "warning": warning
    })
    .to_string())
}
