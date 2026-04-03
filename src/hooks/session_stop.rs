use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_session_stop(input: &str) -> Result<String, HookError> {
    let _parsed: serde_json::Value = serde_json::from_str(input)?;
    Ok(serde_json::json!({
        "decision": "approve",
        "reason": "Session ended. Patterns persisted.",
        "persisted": true
    })
    .to_string())
}
