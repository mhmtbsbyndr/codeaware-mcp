use codeaware_mcp::session::state::{SessionState, SessionPhase};

#[test]
fn test_new_session_is_idle() {
    let state = SessionState::new("/test/project");
    assert_eq!(state.phase(), SessionPhase::Idle);
    assert!(state.session_id().starts_with("s-"));
    assert_eq!(state.steps_completed(), 0);
}

#[test]
fn test_transition_to_analyzing_on_read() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    assert_eq!(state.phase(), SessionPhase::Analyzing);
    assert_eq!(state.steps_completed(), 1);
}

#[test]
fn test_transition_to_editing() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    state.on_smart_edit("src/main.rs");
    assert_eq!(state.phase(), SessionPhase::Editing);
    assert_eq!(state.steps_completed(), 2);
}

#[test]
fn test_transition_to_verifying() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    state.on_smart_edit("src/main.rs");
    state.on_smart_run("cargo test");
    assert_eq!(state.phase(), SessionPhase::Verifying);
}

#[test]
fn test_transition_to_complete() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    state.on_smart_edit("src/main.rs");
    state.on_smart_run("cargo test");
    state.on_task_complete();
    assert_eq!(state.phase(), SessionPhase::Complete);
}

#[test]
fn test_transition_to_compacting() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    state.on_pre_compact();
    assert_eq!(state.phase(), SessionPhase::Compacting);
    state.on_post_compact();
    // After compact, goes back to previous state (or Idle)
    assert_eq!(state.phase(), SessionPhase::Analyzing);
}

#[test]
fn test_edit_in_idle_warns_but_works() {
    let mut state = SessionState::new("/test/project");
    let warnings = state.on_smart_edit("src/main.rs");
    assert_eq!(state.phase(), SessionPhase::Editing);
    assert!(warnings.iter().any(|w| w.contains("Kein vorheriger Read")));
}

#[test]
fn test_files_tracking() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    state.on_smart_read("src/lib.rs");
    state.on_smart_edit("src/main.rs");
    assert_eq!(state.files_read().len(), 2);
    assert_eq!(state.files_edited().len(), 1);
}

#[test]
fn test_error_loop_detection() {
    let mut state = SessionState::new("/test/project");
    state.record_error_signature("abc123");
    state.record_error_signature("abc123");
    state.record_error_signature("abc123");
    assert!(state.is_error_loop("abc123", 3));
    assert!(!state.is_error_loop("abc123", 4));
}

#[test]
fn test_cwd_changed_resets() {
    let mut state = SessionState::new("/test/project");
    state.on_smart_read("src/main.rs");
    assert_eq!(state.phase(), SessionPhase::Analyzing);
    state.on_cwd_changed("/test/other-project");
    assert_eq!(state.phase(), SessionPhase::Idle);
    assert_eq!(state.project_path(), "/test/other-project");
}
