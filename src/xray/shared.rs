//! Shared metrics file for communication between hook processes and xray server.
//! Hooks write `xray.json` into a per-user directory (`$XDG_RUNTIME_DIR/codeaware`,
//! or `~/.codeaware`), the xray server merges it on each SSE tick.
//! Uses flock for safe concurrent access from multiple hook processes.
//!
//! The directory is created with mode 0700 and must be owned by the current user.
//! Files are created with mode 0600 and temp files get random names with
//! `create_new`, so a local attacker cannot pre-create or symlink them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const METRICS_FILE_NAME: &str = "xray.json";
const LOCK_FILE_NAME: &str = "xray.lock";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SharedMetrics {
    pub raw_tokens_total: u64,
    pub compressed_tokens_total: u64,
    pub tool_calls: u32,
    pub file_tokens: HashMap<String, u64>,
    pub timeline: Vec<SharedTimelineEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTimelineEvent {
    pub timestamp: String,
    pub tool: String,
    pub file: Option<String>,
    pub raw_tokens: u64,
}

/// Per-user directory for the metrics files. None if no suitable base is set.
fn metrics_dir_from(xdg_runtime: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if let Some(xdg) = xdg_runtime.filter(|s| !s.trim().is_empty()) {
        return Some(PathBuf::from(xdg).join("codeaware"));
    }
    home.filter(|h| !h.trim().is_empty())
        .map(|h| PathBuf::from(h).join(".codeaware"))
}

fn metrics_dir() -> Option<PathBuf> {
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok();
    let home = std::env::var("HOME").ok();
    metrics_dir_from(xdg.as_deref(), home.as_deref())
}

/// Create the directory with mode 0700 (or tighten an existing one) and check
/// that it is a real directory owned by the current user.
fn ensure_private_dir(dir: &Path) -> Option<()> {
    if fs::symlink_metadata(dir).is_err() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .ok()?;
    }
    let meta = fs::symlink_metadata(dir).ok()?;
    if !meta.is_dir() {
        return None;
    }
    // SAFETY: geteuid has no preconditions and cannot fail.
    if meta.uid() != unsafe { libc::geteuid() } {
        return None;
    }
    if meta.permissions().mode() & 0o077 != 0 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).ok()?;
    }
    Some(())
}

/// Acquire an exclusive file lock for safe read-modify-write.
fn with_lock<F, R>(dir: &Path, f: F) -> Option<R>
where
    F: FnOnce() -> R,
{
    use std::os::unix::io::AsRawFd;

    let lock_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(dir.join(LOCK_FILE_NAME))
        .ok()?;

    // Acquire exclusive lock (blocking)
    let fd = lock_file.as_raw_fd();
    // SAFETY: fd is a valid open file descriptor owned by lock_file.
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX) };
    if ret != 0 {
        return None;
    }

    let result = f();

    // Release lock
    // SAFETY: same valid fd as above.
    unsafe { libc::flock(fd, libc::LOCK_UN) };
    // lock_file dropped here, also releases

    Some(result)
}

/// Append a tool call to the shared metrics file (called from hooks).
/// Protected by flock to prevent lost updates from concurrent hooks.
pub fn append_tool_call(tool_name: &str, file_path: Option<&str>, raw_tokens: u64, compressed_tokens: u64) {
    let Some(dir) = metrics_dir() else { return };
    if ensure_private_dir(&dir).is_none() {
        return;
    }
    with_lock(&dir, || {
        let mut metrics = read_shared_in(&dir).unwrap_or_default();
        metrics.tool_calls += 1;
        metrics.raw_tokens_total += raw_tokens;
        metrics.compressed_tokens_total += compressed_tokens;
        if let Some(f) = file_path {
            *metrics.file_tokens.entry(f.to_string()).or_insert(0) += raw_tokens;
        }
        metrics.timeline.push(SharedTimelineEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool: tool_name.to_string(),
            file: file_path.map(|s| s.to_string()),
            raw_tokens,
        });
        // Keep timeline bounded
        if metrics.timeline.len() > 100 {
            metrics.timeline = metrics.timeline.split_off(metrics.timeline.len() - 100);
        }
        write_shared_in(&dir, &metrics);
    });
}

/// Read shared metrics from file (public, no lock needed for reads).
pub fn read_shared() -> Option<SharedMetrics> {
    read_shared_in(&metrics_dir()?)
}

fn read_shared_in(dir: &Path) -> Option<SharedMetrics> {
    let data = fs::read_to_string(dir.join(METRICS_FILE_NAME)).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write shared metrics atomically: a uniquely named 0600 temp file, then rename.
fn write_shared_in(dir: &Path, metrics: &SharedMetrics) {
    let Ok(json) = serde_json::to_string(metrics) else {
        return;
    };
    let tmp = dir.join(format!("{METRICS_FILE_NAME}.tmp.{}", uuid::Uuid::new_v4()));
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut f| f.write_all(json.as_bytes()));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
        return;
    }
    if fs::rename(&tmp, dir.join(METRICS_FILE_NAME)).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

/// Reset shared metrics (called at session start).
pub fn reset_shared() {
    if let Some(dir) = metrics_dir() {
        let _ = fs::remove_file(dir.join(METRICS_FILE_NAME));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> SharedTimelineEvent {
        SharedTimelineEvent {
            timestamp: "2026-01-01T00:00:00Z".into(),
            tool: "smart_read".into(),
            file: Some("src/lib.rs".into()),
            raw_tokens: 10,
        }
    }

    #[test]
    fn test_metrics_dir_prefers_xdg_runtime_then_home() {
        assert_eq!(
            metrics_dir_from(Some("/run/user/1000"), Some("/home/u")),
            Some(PathBuf::from("/run/user/1000/codeaware"))
        );
        assert_eq!(
            metrics_dir_from(None, Some("/home/u")),
            Some(PathBuf::from("/home/u/.codeaware"))
        );
    }

    #[test]
    fn test_metrics_dir_none_without_base() {
        assert_eq!(metrics_dir_from(None, None), None);
        assert_eq!(metrics_dir_from(Some(""), Some("")), None);
        assert_eq!(metrics_dir_from(None, Some("  ")), None);
    }

    #[test]
    fn test_private_dir_has_mode_0700() {
        let base = tempfile::TempDir::new().unwrap();
        let dir = base.path().join("codeaware");
        ensure_private_dir(&dir).unwrap();
        let mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn test_written_metrics_file_is_private_and_roundtrips() {
        let base = tempfile::TempDir::new().unwrap();
        let dir = base.path().join("codeaware");
        ensure_private_dir(&dir).unwrap();
        let mut metrics = SharedMetrics::default();
        metrics.tool_calls = 3;
        metrics.timeline.push(sample_event());
        write_shared_in(&dir, &metrics);

        let mode = fs::metadata(dir.join(METRICS_FILE_NAME)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let back = read_shared_in(&dir).unwrap();
        assert_eq!(back.tool_calls, 3);
        assert_eq!(back.timeline.len(), 1);
        // No temp files left behind.
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn test_write_does_not_follow_planted_symlink() {
        // A symlink planted at the metrics path must be replaced, not written through.
        let base = tempfile::TempDir::new().unwrap();
        let dir = base.path().join("codeaware");
        ensure_private_dir(&dir).unwrap();
        let victim = base.path().join("victim.txt");
        fs::write(&victim, "keep me").unwrap();
        std::os::unix::fs::symlink(&victim, dir.join(METRICS_FILE_NAME)).unwrap();

        write_shared_in(&dir, &SharedMetrics::default());

        assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
        assert!(read_shared_in(&dir).is_some());
    }

    #[test]
    fn test_symlinked_dir_is_refused() {
        let base = tempfile::TempDir::new().unwrap();
        let real = base.path().join("real");
        fs::create_dir(&real).unwrap();
        let link = base.path().join("codeaware");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(ensure_private_dir(&link).is_none());
    }
}
