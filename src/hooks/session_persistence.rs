//! Session save/resume — persist workspace state to ~/.codeaware/sessions/{session_id}.json
//! and reload it at session start if a recent session for the same project exists.

use serde_json::Value;

/// Save session state to disk.
pub fn save_session_state(session_id: &str, project: &str, state: &Value) {
    let dir = format!(
        "{}/.codeaware/sessions",
        std::env::var("HOME").unwrap_or_default()
    );
    if std::fs::create_dir_all(&dir).is_err() {
        eprintln!("CodeAware: could not create sessions dir: {dir}");
        return;
    }
    let path = format!("{}/{}.json", dir, session_id);
    let payload = serde_json::json!({
        "session_id": session_id,
        "project": project,
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "state": state,
    });
    let content = match serde_json::to_string_pretty(&payload) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("CodeAware: failed to serialize session state: {e}");
            return;
        }
    };

    if let Err(e) = std::fs::write(&path, content) {
        eprintln!("CodeAware: failed to save session state: {e}");
    }
}

/// Load the most recent session state for a given project.
pub fn load_recent_session(project: &str) -> Option<Value> {
    let dir = format!(
        "{}/.codeaware/sessions",
        std::env::var("HOME").unwrap_or_default()
    );
    let entries = std::fs::read_dir(&dir).ok()?;

    let mut best: Option<(String, Value)> = None; // (saved_at, state)

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("CodeAware: failed to read session file {}: {err}", path.display());
                continue;
            }
        };
        let parsed: Value = match serde_json::from_str(&content) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("CodeAware: failed to parse session file {}: {err}", path.display());
                continue;
            }
        };

        // Check project match
        if parsed.get("project").and_then(|p| p.as_str()) != Some(project) {
            continue;
        }

        let saved_at = parsed
            .get("saved_at")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        match &best {
            Some((prev_time, _)) if saved_at > *prev_time => {
                best = Some((saved_at, parsed));
            }
            None => {
                best = Some((saved_at, parsed));
            }
            _ => {}
        }
    }

    best.and_then(|(_, v)| v.get("state").cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_recent_session_no_dir() {
        // When HOME points to a non-existent path, should return None gracefully
        std::env::set_var("HOME", "/tmp/codeaware_test_nonexistent_dir_12345");
        let result = load_recent_session("test-project");
        assert!(result.is_none());
        std::env::remove_var("HOME");
    }
}
