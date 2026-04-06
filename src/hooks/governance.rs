//! Governance capture — log security-relevant and breaking changes.
//!
//! Activated only when `CODEAWARE_GOVERNANCE=1` env var is set.

pub struct GovernanceEvent {
    pub event_type: String,
    pub description: String,
    pub file: Option<String>,
    pub timestamp: String,
}

/// Check whether governance tracking is enabled via env var.
pub fn is_governance_enabled() -> bool {
    std::env::var("CODEAWARE_GOVERNANCE").as_deref() == Ok("1")
}

/// Inspect a tool call result for governance-relevant events.
pub fn check_governance(
    tool_name: &str,
    file_path: Option<&str>,
    result: &str,
) -> Option<GovernanceEvent> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // 1. smart_refactor results → symbol renames
    if tool_name == "smart_refactor" && result.contains("renamed") {
        return Some(GovernanceEvent {
            event_type: "symbol_rename".to_string(),
            description: format!(
                "Symbol rename via smart_refactor on {}",
                file_path.unwrap_or("(project-wide)")
            ),
            file: file_path.map(|s| s.to_string()),
            timestamp,
        });
    }

    // 2. smart_edit on security-sensitive files
    if tool_name == "smart_edit" {
        if let Some(path) = file_path {
            let lower = path.to_lowercase();
            if lower.contains("auth")
                || lower.contains("security")
                || lower.contains(".env")
                || lower.contains("secret")
            {
                return Some(GovernanceEvent {
                    event_type: "security_edit".to_string(),
                    description: format!("Edit on security-sensitive file: {path}"),
                    file: Some(path.to_string()),
                    timestamp,
                });
            }
        }
    }

    // 3. Removal of test files (detected via smart_edit deleting test content)
    if tool_name == "smart_edit" {
        if let Some(path) = file_path {
            let lower = path.to_lowercase();
            if (lower.contains("test") || lower.contains("spec"))
                && result.to_lowercase().contains("removed")
            {
                return Some(GovernanceEvent {
                    event_type: "test_removed".to_string(),
                    description: format!("Potential test removal in: {path}"),
                    file: Some(path.to_string()),
                    timestamp,
                });
            }
        }
    }

    // 4. Config weakening check (see also Fix 5)
    if let Some(event) = check_config_weakening(tool_name, file_path, result) {
        return Some(event);
    }

    None
}

/// Fix 5: Config protection warning — detect edits that weaken configuration files.
pub fn check_config_weakening(
    tool_name: &str,
    file_path: Option<&str>,
    result: &str,
) -> Option<GovernanceEvent> {
    if tool_name != "smart_edit" {
        return None;
    }
    let path = file_path?;
    let lower_path = path.to_lowercase();

    // Only check config files
    let is_config = lower_path.ends_with(".toml")
        || lower_path.ends_with(".json")
        || lower_path.ends_with(".yaml")
        || lower_path.ends_with(".yml")
        || lower_path.contains("eslint")
        || lower_path.contains("prettier")
        || lower_path.contains(".clippy");

    if !is_config {
        return None;
    }

    // Heuristic: check if the edit result suggests weakening
    let lower_result = result.to_lowercase();
    let weakening_keywords = [
        "disabled",
        "\"false\"",
        "'false'",
        "= false",
        ": false",
        "\"off\"",
        "'off'",
        "removed",
        "deleted rule",
        "allow(",
    ];

    for keyword in &weakening_keywords {
        if lower_result.contains(keyword) {
            return Some(GovernanceEvent {
                event_type: "config_weakened".to_string(),
                description: format!(
                    "Config file edit may weaken settings (matched '{keyword}'): {path}"
                ),
                file: Some(path.to_string()),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    None
}

pub fn log_governance_event(event: &GovernanceEvent) {
    eprintln!(
        "\u{26a0} CodeAware Governance: [{}] {} ({})",
        event.event_type,
        event.description,
        event.file.as_deref().unwrap_or("-")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_governance_symbol_rename() {
        let event = check_governance("smart_refactor", Some("src/lib.rs"), "renamed 5 occurrences");
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, "symbol_rename");
    }

    #[test]
    fn test_check_governance_security_edit() {
        let event = check_governance("smart_edit", Some("src/auth.rs"), "updated function");
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, "security_edit");
    }

    #[test]
    fn test_check_governance_no_event_for_normal_edit() {
        let event = check_governance("smart_edit", Some("src/lib.rs"), "updated function");
        assert!(event.is_none());
    }

    #[test]
    fn test_config_weakening_detected() {
        let event = check_config_weakening(
            "smart_edit",
            Some("eslint.config.json"),
            "Rule no-unused-vars set to = false",
        );
        assert!(event.is_some());
        assert_eq!(event.unwrap().event_type, "config_weakened");
    }

    #[test]
    fn test_config_weakening_not_triggered_for_non_config() {
        let event = check_config_weakening(
            "smart_edit",
            Some("src/main.rs"),
            "set to false",
        );
        assert!(event.is_none());
    }
}
