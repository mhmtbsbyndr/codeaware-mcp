use serde_json::{json, Value};
use std::collections::HashMap;
use crate::envelope::{Envelope, ErrorCode, TrustLevel};

// ── WorkspaceSlots ─────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct WorkspaceSlots {
    pub slots: HashMap<String, Value>,
    pub read_count: HashMap<String, u32>,
}

impl WorkspaceSlots {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> (Value, bool) {
        let content = self.slots.get(name).cloned().unwrap_or(Value::Null);
        let is_full = self.read_count.get(name).copied().unwrap_or(0) == 0;
        (content, is_full)
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.slots.insert(name.to_string(), value);
    }

    pub fn clear(&mut self, name: &str) {
        self.slots.remove(name);
        self.read_count.remove(name);
    }

    pub fn mark_read(&mut self, name: &str) {
        *self.read_count.entry(name.to_string()).or_insert(0) += 1;
    }
}

// ── Valid slots ────────────────────────────────────────────────────────────────

const VALID_SLOTS: &[&str] = &[
    "recent_targets",
    "error_signatures",
    "co_access_candidates",
    "verification_state",
    "active_task",
];

fn is_valid_slot(slot: &str) -> bool {
    VALID_SLOTS.contains(&slot)
}

// ── Schema validation ─────────────────────────────────────────────────────────

/// Returns true when `value` loosely matches the expected shape for `slot`.
fn validate_slot_value(slot: &str, value: &Value) -> bool {
    match slot {
        "recent_targets" => {
            // { files: [ { path, hash, last_mode, step } ] }
            value.get("files").and_then(|f| f.as_array()).is_some()
        }
        "error_signatures" => {
            // { errors: [ { hash, count, last_seen, typical_fix? } ] }
            value.get("errors").and_then(|e| e.as_array()).is_some()
        }
        "co_access_candidates" => {
            // { pairs: [ { a, b, frequency } ] }
            value.get("pairs").and_then(|p| p.as_array()).is_some()
        }
        "verification_state" => {
            // { files_modified, files_with_tests, uncommitted, compiler_errors, open_failures }
            value.is_object()
                && value.get("files_modified").is_some()
                && value.get("uncommitted").is_some()
        }
        "active_task" => {
            // { description, state, started_step }
            value.is_object()
                && value.get("description").is_some()
                && value.get("state").is_some()
        }
        _ => false,
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub fn handle_workspace_state(
    params: &Value,
    state: &std::sync::Arc<std::sync::Mutex<crate::session::state::SessionState>>,
) -> Value {
    let action = match params.get("action").and_then(|a| a.as_str()) {
        Some(a) => a,
        None => {
            let env: Envelope<Value> = Envelope::error(
                ErrorCode::EInvalidSlot,
                false,
                Some("Missing required field: action".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    let slot = match params.get("slot").and_then(|s| s.as_str()) {
        Some(s) => s,
        None => {
            let env: Envelope<Value> = Envelope::error(
                ErrorCode::EInvalidSlot,
                false,
                Some("Missing required field: slot".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    if !is_valid_slot(slot) {
        let env: Envelope<Value> = Envelope::error(
            ErrorCode::EInvalidSlot,
            false,
            Some(format!(
                "Unknown slot '{}'. Valid slots: {}",
                slot,
                VALID_SLOTS.join(", ")
            )),
        );
        return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
    }

    let mut locked = match state.lock() {
        Ok(g) => g,
        Err(_) => {
            let env: Envelope<Value> = Envelope::error(
                ErrorCode::EInternalError,
                true,
                Some("State lock poisoned".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    match action {
        "read" => {
            let (content, is_full) = locked.workspace_slots.get(slot);
            locked.workspace_slots.mark_read(slot);
            let data = json!({
                "slot": slot,
                "full": is_full,
                "content": content,
            });
            let env = Envelope::success(data, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        "write" => {
            let value = match params.get("value") {
                Some(v) => v.clone(),
                None => {
                    let env: Envelope<Value> = Envelope::error(
                        ErrorCode::EInvalidSlotValue,
                        false,
                        Some("action=write requires a 'value' field".to_string()),
                    );
                    return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
                }
            };

            if !validate_slot_value(slot, &value) {
                let env: Envelope<Value> = Envelope::error(
                    ErrorCode::EInvalidSlotValue,
                    false,
                    Some(format!(
                        "Value does not match expected schema for slot '{}'",
                        slot
                    )),
                );
                return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
            }

            locked.workspace_slots.set(slot, value);
            let data = json!({ "slot": slot, "written": true });
            let env = Envelope::success(data, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        "clear" => {
            locked.workspace_slots.clear(slot);
            let data = json!({ "slot": slot, "cleared": true });
            let env = Envelope::success(data, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        other => {
            let env: Envelope<Value> = Envelope::error(
                ErrorCode::EInvalidSlot,
                false,
                Some(format!(
                    "Unknown action '{}'. Valid actions: read, write, clear",
                    other
                )),
            );
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
    }
}
