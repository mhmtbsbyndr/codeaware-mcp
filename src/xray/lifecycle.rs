use crate::xray::shared::{private_state_dir, read_private_file, with_dir_lock, write_private_file};
use serde_json::Value;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const RESTART_WINDOW_SECS: u64 = 60;
pub const RESTART_MAX_IN_WINDOW: usize = 5;
pub const RESTART_LOG_FILE: &str = "xray-starts.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Binding,
    Ready,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Binding => "binding",
            Phase::Ready => "ready",
        }
    }
}

/// Tracks the startup phase of one xray server instance.
pub struct Lifecycle {
    started: Instant,
    /// None while binding; the time from start to readiness once ready.
    ready_after: Mutex<Option<Duration>>,
}

impl Lifecycle {
    pub fn new() -> Self {
        Lifecycle {
            started: Instant::now(),
            ready_after: Mutex::new(None),
        }
    }

    fn slot(&self) -> MutexGuard<'_, Option<Duration>> {
        self.ready_after.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn mark_ready(&self) {
        let elapsed = self.started.elapsed();
        let mut slot = self.slot();
        if slot.is_none() {
            *slot = Some(elapsed);
        }
    }

    /// Current phase and elapsed milliseconds (frozen at the moment of becoming ready).
    pub fn status(&self) -> (Phase, u64) {
        match *self.slot() {
            Some(ready_after) => (Phase::Ready, ready_after.as_millis() as u64),
            None => (Phase::Binding, self.started.elapsed().as_millis() as u64),
        }
    }
}

/// HTTP status code and JSON body (without the service name) for `/api/ready`.
pub fn ready_report(phase: Phase, elapsed_ms: u64) -> (u16, Value) {
    let ready = phase == Phase::Ready;
    let code = if ready { 200 } else { 503 };
    let body = serde_json::json!({
        "ready": ready,
        "phase": phase.as_str(),
        "elapsed_ms": elapsed_ms,
    });
    (code, body)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartDecision {
    Allow,
    Refuse { starts_in_window: usize },
}

/// Pure rate-limit decision. `history` holds unix-second timestamps of earlier starts.
/// Refuses when `max` or more of them fall in the window `(now - window, now]`.
pub fn restart_decision(history: &[u64], now: u64, window_secs: u64, max: usize) -> RestartDecision {
    let window_start = now.saturating_sub(window_secs);
    let starts_in_window = history
        .iter()
        .filter(|&&t| t > window_start && t <= now)
        .count();
    if starts_in_window >= max {
        RestartDecision::Refuse { starts_in_window }
    } else {
        RestartDecision::Allow
    }
}

fn load_history(dir: &Path) -> Vec<u64> {
    read_private_file(dir, RESTART_LOG_FILE)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Check the restart log in `dir` (an already private directory) and record `now`
/// when allowed. Refused starts are not recorded. If the log cannot be locked or
/// written, the start is allowed: the limiter must never block xray on I/O trouble.
pub fn check_and_record_start_in(dir: &Path, now: u64) -> RestartDecision {
    with_dir_lock(dir, || {
        let mut history = load_history(dir);
        let decision = restart_decision(&history, now, RESTART_WINDOW_SECS, RESTART_MAX_IN_WINDOW);
        if decision == RestartDecision::Allow {
            let window_start = now.saturating_sub(RESTART_WINDOW_SECS);
            history.retain(|&t| t > window_start && t <= now);
            history.push(now);
            if let Ok(json) = serde_json::to_string(&history) {
                write_private_file(dir, RESTART_LOG_FILE, &json);
            }
        }
        decision
    })
    .unwrap_or(RestartDecision::Allow)
}

/// Rate-limit check for a new xray listener, using the per-user state directory.
pub fn check_and_record_start() -> RestartDecision {
    let Some(dir) = private_state_dir() else {
        return RestartDecision::Allow;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    check_and_record_start_in(&dir, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_allows_up_to_max_starts_in_window() {
        let now = 1_000;
        assert_eq!(
            restart_decision(&[990, 991, 992, 993], now, 60, 5),
            RestartDecision::Allow
        );
        assert_eq!(
            restart_decision(&[990, 991, 992, 993, 994], now, 60, 5),
            RestartDecision::Refuse { starts_in_window: 5 }
        );
    }

    #[test]
    fn restart_ignores_starts_outside_window() {
        // 940 is exactly one window old and falls outside (now - 60, now].
        assert_eq!(
            restart_decision(&[900, 920, 930, 940, 960], 1_000, 60, 5),
            RestartDecision::Allow
        );
    }

    #[test]
    fn check_and_record_limits_burst_and_recovers() {
        let base = tempfile::TempDir::new().unwrap();
        let dir = base.path();
        for i in 0..5 {
            assert_eq!(check_and_record_start_in(dir, 1_000 + i), RestartDecision::Allow);
        }
        assert!(matches!(
            check_and_record_start_in(dir, 1_005),
            RestartDecision::Refuse { .. }
        ));
        // Refused attempts are not recorded, so the block lifts once the window passes.
        assert!(matches!(
            check_and_record_start_in(dir, 1_006),
            RestartDecision::Refuse { .. }
        ));
        assert_eq!(check_and_record_start_in(dir, 1_065), RestartDecision::Allow);
    }

    #[cfg(unix)]
    #[test]
    fn restart_log_is_private_and_not_symlinked() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::TempDir::new().unwrap();
        let dir = base.path().join("codeaware");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let victim = base.path().join("victim.json");
        std::fs::write(&victim, "keep").unwrap();
        std::os::unix::fs::symlink(&victim, dir.join(RESTART_LOG_FILE)).unwrap();

        assert_eq!(check_and_record_start_in(&dir, 1_000), RestartDecision::Allow);

        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
        let mode = std::fs::metadata(dir.join(RESTART_LOG_FILE)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn ready_report_is_503_while_binding_and_200_when_ready() {
        let (code, body) = ready_report(Phase::Binding, 3);
        assert_eq!(code, 503);
        assert_eq!(body["ready"], false);
        assert_eq!(body["phase"], "binding");
        assert_eq!(body["elapsed_ms"], 3);

        let (code, body) = ready_report(Phase::Ready, 12);
        assert_eq!(code, 200);
        assert_eq!(body["ready"], true);
        assert_eq!(body["phase"], "ready");
    }

    #[test]
    fn lifecycle_freezes_elapsed_time_once_ready() {
        let lc = Lifecycle::new();
        assert_eq!(lc.status().0, Phase::Binding);
        lc.mark_ready();
        let (phase, first) = lc.status();
        std::thread::sleep(Duration::from_millis(20));
        let (_, second) = lc.status();
        assert_eq!(phase, Phase::Ready);
        assert_eq!(first, second);
    }
}
