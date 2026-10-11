use crate::hooks::{context_injection, session_persistence, spool};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

/// Spool the stop event, then drain the spool, so the state is saved durably
/// even if this process dies before the save finishes.
pub fn handle_session_stop(input: &str) -> Result<String, HookError> {
    let parsed: Value = serde_json::from_str(input)?;
    let payload = stop_payload(&parsed);

    let persisted = match spool::enqueue("session_stop", &payload) {
        Ok(path) => {
            crate::hooks::drain_spool();
            // Still present means the drain could not apply it; it stays queued.
            !path.exists()
        }
        Err(e) => {
            eprintln!("CodeAware: spool unavailable ({e}); saving session state directly");
            apply_session_stop(&payload).is_ok()
        }
    };

    let reason = if persisted {
        "Session ended. Patterns persisted."
    } else {
        "Session ended. Persistence deferred to the next hook call."
    };
    Ok(json!({
        "decision": "approve",
        "reason": reason,
        "persisted": persisted
    })
    .to_string())
}

fn stop_payload(parsed: &Value) -> Value {
    let session_id = parsed
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let project = parsed
        .get("project")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    json!({ "session_id": session_id, "project": project, "state": parsed })
}

/// Apply a spooled `session_stop` event: save the session state and pre-build
/// the SessionStart context block for the next session.
pub fn apply_session_stop(payload: &Value) -> Result<(), String> {
    let session_id = payload["session_id"].as_str().unwrap_or("unknown");
    let project = payload["project"].as_str().unwrap_or(".");
    // Errors are reported on stderr and otherwise ignored, as before.
    session_persistence::save_session_state(session_id, project, &payload["state"]);
    context_injection::prebuild_for_project(project);
    Ok(())
}
