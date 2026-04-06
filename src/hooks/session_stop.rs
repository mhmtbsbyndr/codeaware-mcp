use crate::hooks::session_persistence;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_session_stop(input: &str) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;

    // Save session state for resume on next session start
    let session_id = parsed
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let project = parsed
        .get("project")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    session_persistence::save_session_state(session_id, project, &parsed);

    Ok(serde_json::json!({
        "decision": "approve",
        "reason": "Session ended. Patterns persisted.",
        "persisted": true
    })
    .to_string())
}
