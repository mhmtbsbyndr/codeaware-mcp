use crate::hooks::profiles;
use crate::session::persistence::SessionDb;

/// Load relevant memories from previous sessions and return formatted context.
/// The returned string is logged to stderr so it appears in the MCP server startup log
/// visible to the LLM, and can also be stored in SessionState for session_status.
pub fn inject_context(db: &SessionDb, project_path: &str) -> Option<String> {
    // Check if this hook is disabled via CODEAWARE_DISABLED_HOOKS
    if profiles::is_hook_disabled("context_injection") {
        return None;
    }

    let observations = match db.get_recent_observations_for_project(project_path, 10) {
        Ok(obs) => obs,
        Err(_) => return None,
    };

    if observations.is_empty() {
        return None;
    }

    // Format observations as a compact text block
    let mut lines = Vec::with_capacity(observations.len() + 1);
    lines.push(format!(
        "CodeAware: {} memories from previous sessions:",
        observations.len()
    ));

    for obs in &observations {
        let title = obs
            .title
            .as_deref()
            .unwrap_or("(untitled)");
        let obs_type = &obs.observation_type;
        let text_preview: String = obs
            .text
            .chars()
            .take(100)
            .collect::<String>()
            .replace('\n', " ");
        lines.push(format!("  [{}] {} — {}", obs_type, title, text_preview));
    }

    let formatted = lines.join("\n");

    // Log to stderr so it appears in MCP server startup log visible to the LLM
    eprintln!("{}", formatted);

    Some(formatted)
}
