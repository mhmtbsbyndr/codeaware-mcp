#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    pub code: String,
    pub severity: SecuritySeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityPolicy {
    pub deny_commands: Vec<String>,
    pub deny_paths: Vec<String>,
    pub redact_secrets: bool,
    pub allow_network_access: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            deny_commands: vec![
                "rm -rf /".to_string(),
                "shutdown".to_string(),
            ],
            deny_paths: vec!["/etc".to_string()],
            redact_secrets: true,
            allow_network_access: false,
        }
    }
}

impl SecurityPolicy {
    pub fn validate_command(&self, command: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        for denied in &self.deny_commands {
            if command.contains(denied) {
                findings.push(SecurityFinding {
                    code: "SEC-CMD-001".to_string(),
                    severity: SecuritySeverity::Critical,
                    message: format!("Denied command pattern detected: {}", denied),
                });
            }
        }

        findings
    }

    pub fn validate_path(&self, path: &str) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();

        for denied in &self.deny_paths {
            if path.starts_with(denied) {
                findings.push(SecurityFinding {
                    code: "SEC-PATH-001".to_string(),
                    severity: SecuritySeverity::Critical,
                    message: format!("Denied path access: {}", denied),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_denied_command() {
        let policy = SecurityPolicy::default();
        let findings = policy.validate_command("rm -rf /");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "SEC-CMD-001");
    }

    #[test]
    fn detects_denied_path() {
        let policy = SecurityPolicy::default();
        let findings = policy.validate_path("/etc/passwd");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "SEC-PATH-001");
    }
}
