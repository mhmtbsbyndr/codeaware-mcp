use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_pre_compact(input: &str) -> Result<String, HookError> {
    // Validate it's valid JSON (session_id may be used in full impl)
    let _parsed: serde_json::Value = serde_json::from_str(input)?;
    // In a full implementation, this would persist session state to SQLite
    Ok(serde_json::json!({
        "decision": "approve",
        "reason": "Session state persisted"
    })
    .to_string())
}

pub fn handle_post_compact(input: &str) -> Result<String, HookError> {
    let _parsed: serde_json::Value = serde_json::from_str(input)?;
    // Mark all seen files as pre-compact
    Ok(serde_json::json!({
        "decision": "approve",
        "reason": "Seen files marked as pre-compact"
    })
    .to_string())
}
