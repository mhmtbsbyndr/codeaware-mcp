//! Session save/resume — persist workspace state to ~/.codeaware/sessions/{session_id}.json
//! and reload it at session start if a recent session for the same project exists.

use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_SESSION_ID_LEN: usize = 128;

/// The session id comes from hook JSON and becomes a file name, so only a
/// conservative character set is accepted (no separators, no dots).
pub fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_LEN {
        return Err(format!(
            "invalid session id length (must be 1..={MAX_SESSION_ID_LEN} characters)"
        ));
    }
    if !session_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("invalid session id: only [A-Za-z0-9_-] is allowed".into());
    }
    Ok(())
}

/// Sessions directory below `home`. An empty HOME is an error, never "/.codeaware".
fn sessions_dir_from_home(home: &str) -> Result<PathBuf, String> {
    if home.trim().is_empty() {
        return Err("HOME is not set; cannot determine ~/.codeaware/sessions".into());
    }
    Ok(PathBuf::from(home).join(".codeaware").join("sessions"))
}

fn sessions_dir() -> Result<PathBuf, String> {
    sessions_dir_from_home(&std::env::var("HOME").unwrap_or_default())
}

/// Save session state, returning the written path or an error.
pub fn try_save_session_state(session_id: &str, project: &str, state: &Value) -> Result<PathBuf, String> {
    validate_session_id(session_id)?;
    let dir = sessions_dir()?;
    save_session_state_in(&dir, session_id, project, state)
}

/// Save session state to disk. Errors are reported on stderr and otherwise ignored.
pub fn save_session_state(session_id: &str, project: &str, state: &Value) {
    if let Err(e) = try_save_session_state(session_id, project, state) {
        eprintln!("CodeAware: failed to save session state: {e}");
    }
}

fn save_session_state_in(
    dir: &Path,
    session_id: &str,
    project: &str,
    state: &Value,
) -> Result<PathBuf, String> {
    validate_session_id(session_id)?;
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create sessions dir {}: {e}", dir.display()))?;
    let path = dir.join(format!("{session_id}.json"));
    let payload = serde_json::json!({
        "session_id": session_id,
        "project": project,
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "state": state,
    });
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("failed to serialize session state: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    Ok(path)
}

/// Load the most recent session state for a given project.
pub fn load_recent_session(project: &str) -> Option<Value> {
    let dir = sessions_dir().ok()?;
    load_recent_session_in(&dir, project)
}

fn load_recent_session_in(dir: &Path, project: &str) -> Option<Value> {
    let entries = std::fs::read_dir(dir).ok()?;

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

    #[test]
    fn test_session_id_rejects_path_traversal() {
        for bad in ["../../x", "a/b", "a\\b", "..", "x.y", "", "has space", "nul\0byte"] {
            assert!(validate_session_id(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn test_session_id_length_limit() {
        assert!(validate_session_id(&"a".repeat(128)).is_ok());
        assert!(validate_session_id(&"a".repeat(129)).is_err());
    }

    #[test]
    fn test_session_id_accepts_safe_names() {
        assert!(validate_session_id("abc-DEF_123").is_ok());
        assert!(validate_session_id("3f2c9a1e-7b4d-4e8a-9c2b-1a2b3c4d5e6f").is_ok());
    }

    #[test]
    fn test_empty_home_is_an_error() {
        assert!(sessions_dir_from_home("").is_err());
        assert!(sessions_dir_from_home("   ").is_err());
        assert_eq!(
            sessions_dir_from_home("/home/u").unwrap(),
            PathBuf::from("/home/u/.codeaware/sessions")
        );
    }

    #[test]
    fn test_save_rejects_traversal_and_writes_nothing() {
        let base = tempfile::TempDir::new().unwrap();
        let sessions = base.path().join("sessions");
        let state = serde_json::json!({"x": 1});
        assert!(save_session_state_in(&sessions, "../../escape", "p", &state).is_err());
        assert!(!base.path().join("escape.json").exists());
        assert!(!sessions.exists());
    }

    #[test]
    fn test_save_then_load_roundtrip() {
        let base = tempfile::TempDir::new().unwrap();
        let sessions = base.path().join("sessions");
        let state = serde_json::json!({"task": "fix bug"});
        let path = save_session_state_in(&sessions, "sess-1", "proj", &state).unwrap();
        assert_eq!(path, sessions.join("sess-1.json"));
        assert_eq!(load_recent_session_in(&sessions, "proj"), Some(state));
        assert!(load_recent_session_in(&sessions, "other").is_none());
    }
}
