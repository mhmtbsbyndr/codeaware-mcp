pub mod auto_observe;
pub mod compact;
pub mod context_cache;
pub mod context_injection;
pub mod governance;
pub mod post_tool_use;
pub mod profiles;
pub mod session_persistence;
pub mod session_stop;
pub mod spool;
pub mod stdin_utf8;
pub mod subagent_stop;
pub mod tail_reader;
pub mod tool_failure;

use crate::xray::shared;
use serde_json::Value;

/// Apply one spooled event. Unknown kinds are an error and stay queued.
fn apply_spooled(kind: &str, payload: &Value) -> Result<(), String> {
    match kind {
        "observation" | "file_edit" => {
            let tool_name = payload["tool_name"].as_str().unwrap_or("unknown");
            let file_path = payload["file_path"].as_str();
            let estimated = payload["estimated_tokens"].as_u64().unwrap_or(0);
            let compressed = payload["compressed_estimate"].as_u64().unwrap_or(0);
            shared::append_tool_call(tool_name, file_path, estimated, compressed);
            if kind == "file_edit" {
                let project = payload["project"].as_str().unwrap_or(".");
                let _ = context_cache::mark_stale(project);
            }
            Ok(())
        }
        "session_stop" => session_stop::apply_session_stop(payload),
        other => Err(format!("unknown spooled event kind: {other}")),
    }
}

/// Process events that earlier hook calls spooled. Called before each hook.
pub fn drain_spool() -> spool::DrainReport {
    spool::drain(apply_spooled)
}

/// Dispatch hook by event name. Called from main.rs CLI.
pub fn dispatch_hook(event: &str, stdin_json: &str) -> Result<String, String> {
    drain_spool();
    match event {
        "PostToolUse" | "post-tool-use" => post_tool_use::handle_post_tool_use(stdin_json).map_err(|e| e.to_string()),
        "PostToolUseFailure" | "tool-failure" => tool_failure::handle_tool_failure(stdin_json).map_err(|e| e.to_string()),
        "PreCompact" | "pre-compact" => compact::handle_pre_compact(stdin_json).map_err(|e| e.to_string()),
        "PostCompact" | "post-compact" => compact::handle_post_compact(stdin_json).map_err(|e| e.to_string()),
        "SubagentStop" | "subagent-stop" => subagent_stop::handle_subagent_stop(stdin_json).map_err(|e| e.to_string()),
        "Stop" | "SessionEnd" | "session-stop" => session_stop::handle_session_stop(stdin_json).map_err(|e| e.to_string()),
        _ => Err(format!("Unknown hook event: {event}")),
    }
}
