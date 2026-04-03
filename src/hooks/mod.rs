pub mod compact;
pub mod post_tool_use;
pub mod session_stop;
pub mod subagent_stop;
pub mod tool_failure;

/// Dispatch hook by event name. Called from main.rs CLI.
pub fn dispatch_hook(event: &str, stdin_json: &str) -> Result<String, String> {
    match event {
        "post-tool-use" => post_tool_use::handle_post_tool_use(stdin_json).map_err(|e| e.to_string()),
        "tool-failure" => tool_failure::handle_tool_failure(stdin_json).map_err(|e| e.to_string()),
        "pre-compact" => compact::handle_pre_compact(stdin_json).map_err(|e| e.to_string()),
        "post-compact" => compact::handle_post_compact(stdin_json).map_err(|e| e.to_string()),
        "subagent-stop" => subagent_stop::handle_subagent_stop(stdin_json).map_err(|e| e.to_string()),
        "session-stop" => session_stop::handle_session_stop(stdin_json).map_err(|e| e.to_string()),
        _ => Err(format!("Unknown hook event: {event}")),
    }
}
