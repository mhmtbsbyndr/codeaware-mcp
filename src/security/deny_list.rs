use regex::Regex;
use std::path::Path;

pub struct DenyList {
    read_patterns: Vec<Regex>,
    edit_patterns: Vec<Regex>,
    command_patterns: Vec<Regex>,
}

impl Default for DenyList {
    fn default() -> Self {
        let read_patterns = vec![
            // .env and .env.* files
            Regex::new(r"(?i)^\.env$").unwrap(),
            Regex::new(r"(?i)^\.env\.").unwrap(),
            // secrets/ directory
            Regex::new(r"(?i)(^|/)secrets/").unwrap(),
            // credentials.*
            Regex::new(r"(?i)(^|/)credentials\.").unwrap(),
            // .pem files
            Regex::new(r"(?i)\.pem$").unwrap(),
            // .key files
            Regex::new(r"(?i)\.key$").unwrap(),
            // node_modules/
            Regex::new(r"(?i)(^|/)node_modules/").unwrap(),
            // target/
            Regex::new(r"(?i)(^|/)target/").unwrap(),
        ];

        let edit_patterns = vec![
            // Binary formats
            Regex::new(r"(?i)\.wasm$").unwrap(),
            Regex::new(r"(?i)\.so$").unwrap(),
            Regex::new(r"(?i)\.dylib$").unwrap(),
            Regex::new(r"(?i)\.exe$").unwrap(),
            Regex::new(r"(?i)\.dll$").unwrap(),
            // Generated files
            Regex::new(r"(?i)\.generated\.").unwrap(),
            // Lock files
            Regex::new(r"(?i)(^|/)Cargo\.lock$").unwrap(),
            Regex::new(r"(?i)(^|/)package-lock\.json$").unwrap(),
            Regex::new(r"(?i)(^|/)yarn\.lock$").unwrap(),
        ];

        let command_patterns = vec![
            // rm -rf / or rm -rf .
            Regex::new(r"rm\s+-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*\s+[/.]").unwrap(),
            Regex::new(r"rm\s+-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*\s+[/.]").unwrap(),
            // curl piped to sh
            Regex::new(r"curl\s+.*\|\s*(sh|bash|zsh|fish|dash)").unwrap(),
            // wget piped to bash
            Regex::new(r"wget\s+.*\|\s*(sh|bash|zsh|fish|dash)").unwrap(),
            // sudo
            Regex::new(r"(?:^|\s)sudo\s").unwrap(),
        ];

        DenyList {
            read_patterns,
            edit_patterns,
            command_patterns,
        }
    }
}

impl DenyList {
    /// Returns true if reading the given path should be denied.
    /// Matches against both the full path and the filename component.
    pub fn is_read_denied(&self, path: &str) -> bool {
        let filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        self.read_patterns
            .iter()
            .any(|re| re.is_match(path) || re.is_match(filename))
    }

    /// Returns true if editing the given path should be denied.
    /// Matches against both the full path and the filename component.
    pub fn is_edit_denied(&self, path: &str) -> bool {
        let filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        self.edit_patterns
            .iter()
            .any(|re| re.is_match(path) || re.is_match(filename))
    }

    /// Returns true if running the given command should be denied.
    /// Matches against the full command string.
    pub fn is_command_denied(&self, command: &str) -> bool {
        self.command_patterns.iter().any(|re| re.is_match(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let deny = DenyList::default();
        assert!(deny.is_read_denied(".env"));
        assert!(!deny.is_read_denied("src/main.rs"));
    }
}
