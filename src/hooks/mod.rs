pub mod auto_observe;
pub mod compact;
pub mod context_injection;
pub mod post_tool_use;
pub mod session_stop;
pub mod subagent_stop;
pub mod tool_failure;

/// Dispatch hook by event name. Called from main.rs CLI.
pub fn dispatch_hook(event: &str, stdin_json: &str) -> Result<String, String> {
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
