use crate::hooks::{context_cache, profiles};
use crate::session::persistence::{ObservationRecord, SessionDb};

const CONTEXT_LIMIT: usize = 10;

/// Load relevant memories from previous sessions and return formatted context.
/// The returned string is logged to stderr so it appears in the MCP server startup log
/// visible to the LLM, and can also be stored in SessionState for session_status.
///
/// A block pre-built on session stop is reused when it is fresh, its source
/// fingerprint matches the current observations, and no stale marker is set.
/// Otherwise the block is formatted live and written back to the cache.
pub fn inject_context(db: &SessionDb, project_path: &str) -> Option<String> {
    // Check if this hook is disabled via CODEAWARE_DISABLED_HOOKS
    if profiles::is_hook_disabled("context_injection") {
        return None;
    }

    let observations = observation_snapshot(db, project_path)?;
    let fingerprint = context_cache::fingerprint(
        observations.iter().map(|o| (o.id, o.updated_at.as_str())),
    );

    let formatted = match context_cache::read_fresh(project_path, &fingerprint) {
        Some(cached) => cached,
        None => {
            let live = format_observations(&observations);
            let _ = context_cache::write(project_path, &fingerprint, &live);
            live
        }
    };

    // Log to stderr so it appears in MCP server startup log visible to the LLM
    eprintln!("{}", formatted);

    Some(formatted)
}

/// Format the context block ahead of time and store it in the context cache.
/// Used by the session stop hook. Returns true if a block was written.
pub fn prebuild_for_project(project_path: &str) -> bool {
    if profiles::is_hook_disabled("context_injection") {
        return false;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if home.trim().is_empty() {
        return false;
    }
    let db_path = std::path::Path::new(&home).join(".codeaware").join("session.db");
    if !db_path.exists() {
        return false;
    }
    let Ok(db) = SessionDb::open(&db_path) else {
        return false;
    };
    let Some(observations) = observation_snapshot(&db, project_path) else {
        return false;
    };
    let fingerprint = context_cache::fingerprint(
        observations.iter().map(|o| (o.id, o.updated_at.as_str())),
    );
    let text = format_observations(&observations);
    context_cache::write(project_path, &fingerprint, &text).is_ok()
}

/// Recent observations for a project; None when the query fails or there are none.
fn observation_snapshot(db: &SessionDb, project_path: &str) -> Option<Vec<ObservationRecord>> {
    match db.get_recent_observations_for_project(project_path, CONTEXT_LIMIT) {
        Ok(obs) if !obs.is_empty() => Some(obs),
        _ => None,
    }
}

fn format_observations(observations: &[ObservationRecord]) -> String {
    // Format observations as a compact text block
    let mut lines = Vec::with_capacity(observations.len() + 1);
    lines.push(format!(
        "CodeAware: {} memories from previous sessions:",
        observations.len()
    ));

    for obs in observations {
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

    lines.join("\n")
}
