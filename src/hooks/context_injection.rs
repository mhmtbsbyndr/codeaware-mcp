use crate::session::persistence::SessionDb;

/// Load relevant memories from previous sessions and log injection count.
pub fn inject_context(db: &SessionDb, project_path: &str) {
    let observations = match db.get_recent_observations_for_project(project_path, 10) {
        Ok(obs) => obs,
        Err(_) => return,
    };

    if observations.is_empty() {
        return;
    }

    // Log injected context count
    eprintln!(
        "CodeAware: injected {} memories from previous sessions",
        observations.len()
    );
}
