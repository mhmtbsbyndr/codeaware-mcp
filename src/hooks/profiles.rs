/// Runtime hook profiles controlled by environment variables.
///
/// - `CODEAWARE_PROFILE=minimal|standard|rich` (default: standard)
/// - `CODEAWARE_DISABLED_HOOKS=auto_observe,context_injection` to disable specific hooks

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Minimal,
    Standard,
    Rich,
}

pub fn get_profile() -> Profile {
    match std::env::var("CODEAWARE_PROFILE").as_deref() {
        Ok("minimal") => Profile::Minimal,
        Ok("rich") => Profile::Rich,
        _ => Profile::Standard,
    }
}

pub fn is_hook_disabled(hook_name: &str) -> bool {
    std::env::var("CODEAWARE_DISABLED_HOOKS")
        .map(|v| v.split(',').any(|h| h.trim() == hook_name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_is_standard() {
        // When env var is not set, default to Standard
        std::env::remove_var("CODEAWARE_PROFILE");
        assert_eq!(get_profile(), Profile::Standard);
    }

    #[test]
    fn test_is_hook_disabled_when_unset() {
        std::env::remove_var("CODEAWARE_DISABLED_HOOKS");
        assert!(!is_hook_disabled("auto_observe"));
    }
}
