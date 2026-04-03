use codeaware_mcp::tools::workspace_state::{WorkspaceSlots, handle_workspace_state};
use codeaware_mcp::session::state::SessionState;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn make_state() -> Arc<Mutex<SessionState>> {
    Arc::new(Mutex::new(SessionState::new("/tmp/test")))
}

// Test 1: write and read active_task
#[test]
fn test_write_and_read_active_task() {
    let state = make_state();

    let write_result = handle_workspace_state(
        &json!({
            "action": "write",
            "slot": "active_task",
            "value": {
                "description": "Implement workspace_state tool",
                "state": "in_progress",
                "started_step": 1
            }
        }),
        &state,
    );

    assert_eq!(write_result["ok"], true, "write should succeed");

    let read_result = handle_workspace_state(
        &json!({
            "action": "read",
            "slot": "active_task"
        }),
        &state,
    );

    assert_eq!(read_result["ok"], true, "read should succeed");
    let data = &read_result["data"];
    assert_eq!(data["slot"], "active_task");
    assert_eq!(
        data["content"]["description"],
        "Implement workspace_state tool"
    );
    assert_eq!(data["content"]["state"], "in_progress");
}

// Test 2: invalid slot returns E_INVALID_SLOT
#[test]
fn test_invalid_slot_returns_error() {
    let state = make_state();

    let result = handle_workspace_state(
        &json!({
            "action": "read",
            "slot": "nonexistent_slot"
        }),
        &state,
    );

    assert_eq!(result["ok"], false, "invalid slot should fail");
    assert_eq!(result["error_code"], "E_INVALID_SLOT");
}

// Test 3: clear removes slot
#[test]
fn test_clear_removes_slot() {
    let state = make_state();

    // Write first
    handle_workspace_state(
        &json!({
            "action": "write",
            "slot": "active_task",
            "value": {
                "description": "Some task",
                "state": "pending",
                "started_step": 0
            }
        }),
        &state,
    );

    // Clear it
    let clear_result = handle_workspace_state(
        &json!({
            "action": "clear",
            "slot": "active_task"
        }),
        &state,
    );
    assert_eq!(clear_result["ok"], true, "clear should succeed");

    // Read should return null content
    let read_result = handle_workspace_state(
        &json!({
            "action": "read",
            "slot": "active_task"
        }),
        &state,
    );
    assert_eq!(read_result["ok"], true);
    assert!(
        read_result["data"]["content"].is_null(),
        "content should be null after clear"
    );
}

// Test 4: first read returns full:true, second read returns full:false
#[test]
fn test_first_read_full_second_read_not_full() {
    let state = make_state();

    // Write a value first
    handle_workspace_state(
        &json!({
            "action": "write",
            "slot": "verification_state",
            "value": {
                "files_modified": 3,
                "files_with_tests": 2,
                "uncommitted": 1,
                "compiler_errors": 0,
                "open_failures": 0
            }
        }),
        &state,
    );

    // First read: full should be true
    let first = handle_workspace_state(
        &json!({ "action": "read", "slot": "verification_state" }),
        &state,
    );
    assert_eq!(first["ok"], true);
    assert_eq!(first["data"]["full"], true, "first read should be full");

    // Second read: full should be false
    let second = handle_workspace_state(
        &json!({ "action": "read", "slot": "verification_state" }),
        &state,
    );
    assert_eq!(second["ok"], true);
    assert_eq!(second["data"]["full"], false, "second read should not be full");
}

// Test 5: invalid value for slot returns E_INVALID_SLOT_VALUE
#[test]
fn test_invalid_value_returns_error() {
    let state = make_state();

    // active_task requires 'description' and 'state' fields
    let result = handle_workspace_state(
        &json!({
            "action": "write",
            "slot": "active_task",
            "value": {
                "wrong_field": "bad data"
            }
        }),
        &state,
    );

    assert_eq!(result["ok"], false, "invalid value should fail");
    assert_eq!(result["error_code"], "E_INVALID_SLOT_VALUE");
}

// Test 6: action=write with no value field returns E_INVALID_SLOT_VALUE
#[test]
fn test_write_missing_value_returns_error() {
    let state = make_state();

    let result = handle_workspace_state(
        &json!({
            "action": "write",
            "slot": "active_task"
        }),
        &state,
    );

    assert_eq!(result["ok"], false, "write without value should fail");
    assert_eq!(result["error_code"], "E_INVALID_SLOT_VALUE");
}

// Test 7: missing action field returns error
#[test]
fn test_missing_action_returns_error() {
    let state = make_state();

    let result = handle_workspace_state(
        &json!({
            "slot": "active_task"
        }),
        &state,
    );

    assert_eq!(result["ok"], false, "missing action should fail");
    assert_eq!(result["error_code"], "E_INVALID_SLOT");
}

// Test 8: missing slot field returns error
#[test]
fn test_missing_slot_returns_error() {
    let state = make_state();

    let result = handle_workspace_state(
        &json!({
            "action": "read"
        }),
        &state,
    );

    assert_eq!(result["ok"], false, "missing slot should fail");
    assert_eq!(result["error_code"], "E_INVALID_SLOT");
}

// Bonus: WorkspaceSlots unit tests
#[test]
fn test_workspace_slots_get_set_clear() {
    let mut slots = WorkspaceSlots::new();

    // Initially null
    let (val, full) = slots.get("recent_targets");
    assert!(val.is_null());
    assert!(full, "first get should be full");

    // Set a value
    slots.set("recent_targets", json!({"files": []}));
    let (val2, _) = slots.get("recent_targets");
    assert_eq!(val2, json!({"files": []}));

    // Mark read, then second get is not full
    slots.mark_read("recent_targets");
    let (_, full2) = slots.get("recent_targets");
    assert!(!full2, "after mark_read full should be false");

    // Clear
    slots.clear("recent_targets");
    let (val3, full3) = slots.get("recent_targets");
    assert!(val3.is_null());
    assert!(full3, "after clear, full resets to true");
}
