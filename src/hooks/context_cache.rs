//! Pre-built SessionStart context cache.
//!
//! The context block for a project is formatted ahead of time (on session stop)
//! and stored under `~/.codeaware/context-cache`. A later SessionStart reads it
//! when three conditions hold:
//!
//! 1. the entry is younger than `TTL`,
//! 2. its source fingerprint matches the current observations, and
//! 3. no stale marker was written for the project (`mark_stale`).
//!
//! Otherwise the caller computes the block live and writes it back.

use blake3::Hasher;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CACHE_DIR_NAME: &str = "context-cache";
/// Entries older than this are recomputed.
pub const TTL: Duration = Duration::from_secs(24 * 60 * 60);

fn cache_dir_from_home(home: &str) -> Result<PathBuf, String> {
    if home.trim().is_empty() {
        return Err("HOME is not set; cannot determine the context cache directory".into());
    }
    Ok(PathBuf::from(home).join(".codeaware").join(CACHE_DIR_NAME))
}

fn default_dir() -> Result<PathBuf, String> {
    cache_dir_from_home(&std::env::var("HOME").unwrap_or_default())
}

/// File-name-safe key for a project path.
pub fn project_key(project: &str) -> String {
    blake3::hash(project.as_bytes()).to_hex()[..32].to_string()
}

/// Fingerprint of a set of observations: a hash over `(id, updated_at)` pairs.
pub fn fingerprint<'a, I>(rows: I) -> String
where
    I: IntoIterator<Item = (i64, &'a str)>,
{
    let mut h = Hasher::new();
    for (id, updated_at) in rows {
        h.update(&id.to_le_bytes());
        h.update(updated_at.as_bytes());
        h.update(&[0u8]);
    }
    h.finalize().to_hex().to_string()
}

/// Read a fresh cached block from the default directory.
pub fn read_fresh(project: &str, fingerprint: &str) -> Option<String> {
    read_fresh_in(&default_dir().ok()?, project, fingerprint, TTL, now_secs())
}

/// Store a block in the default directory.
pub fn write(project: &str, fingerprint: &str, text: &str) -> Result<PathBuf, String> {
    write_in(&default_dir()?, project, fingerprint, text, now_secs())
}

/// Mark the project's cache as stale in the default directory.
pub fn mark_stale(project: &str) -> Result<(), String> {
    mark_stale_in(&default_dir()?, project)
}

/// Read a fresh, matching, unflagged entry from `dir`.
pub fn read_fresh_in(
    dir: &Path,
    project: &str,
    fingerprint: &str,
    ttl: Duration,
    now: u64,
) -> Option<String> {
    todo!("stub")
}

/// Atomically write an entry into `dir` and clear any stale marker.
pub fn write_in(
    dir: &Path,
    project: &str,
    fingerprint: &str,
    text: &str,
    now: u64,
) -> Result<PathBuf, String> {
    todo!("stub")
}

/// Write a stale marker so the next read falls back to live computation.
pub fn mark_stale_in(dir: &Path, project: &str) -> Result<(), String> {
    todo!("stub")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "codeaware-ctx-test-{}-{}",
            name,
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    const DAY: Duration = Duration::from_secs(86_400);

    #[test]
    fn written_entry_is_read_back_when_fresh_and_matching() {
        let dir = tmp_dir("hit");
        write_in(&dir, "/proj", "fp1", "CodeAware: 2 memories", 1_000).unwrap();
        let got = read_fresh_in(&dir, "/proj", "fp1", DAY, 1_100);
        assert_eq!(got.as_deref(), Some("CodeAware: 2 memories"));
    }

    #[test]
    fn stale_entry_by_age_is_not_used() {
        let dir = tmp_dir("ttl");
        write_in(&dir, "/proj", "fp1", "old", 1_000).unwrap();
        assert!(read_fresh_in(&dir, "/proj", "fp1", DAY, 1_000 + 86_401).is_none());
    }

    #[test]
    fn fingerprint_mismatch_is_not_used() {
        let dir = tmp_dir("fp");
        write_in(&dir, "/proj", "fp1", "block", 1_000).unwrap();
        assert!(read_fresh_in(&dir, "/proj", "fp2", DAY, 1_001).is_none());
    }

    #[test]
    fn stale_marker_forces_fallback_until_next_write() {
        let dir = tmp_dir("flag");
        write_in(&dir, "/proj", "fp1", "block", 1_000).unwrap();
        mark_stale_in(&dir, "/proj").unwrap();
        assert!(read_fresh_in(&dir, "/proj", "fp1", DAY, 1_001).is_none());

        write_in(&dir, "/proj", "fp1", "block2", 1_002).unwrap();
        assert_eq!(read_fresh_in(&dir, "/proj", "fp1", DAY, 1_003).as_deref(), Some("block2"));
    }

    #[test]
    fn missing_or_corrupt_entry_returns_none() {
        let dir = tmp_dir("missing");
        assert!(read_fresh_in(&dir, "/nope", "fp", DAY, 0).is_none());
        fs::write(dir.join(format!("{}.json", project_key("/bad"))), b"{ broken").unwrap();
        assert!(read_fresh_in(&dir, "/bad", "fp", DAY, 0).is_none());
    }

    #[test]
    fn projects_are_isolated_and_keys_are_safe_names() {
        let dir = tmp_dir("iso");
        write_in(&dir, "/a", "fp", "A", 10).unwrap();
        write_in(&dir, "/b", "fp", "B", 10).unwrap();
        assert_eq!(read_fresh_in(&dir, "/a", "fp", DAY, 11).as_deref(), Some("A"));
        assert_eq!(read_fresh_in(&dir, "/b", "fp", DAY, 11).as_deref(), Some("B"));
        let key = project_key("../../etc/passwd");
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fingerprint_changes_when_rows_change() {
        let a = fingerprint([(1, "2026-01-01"), (2, "2026-01-02")]);
        let b = fingerprint([(1, "2026-01-01"), (2, "2026-01-03")]);
        let c = fingerprint([(1, "2026-01-01"), (2, "2026-01-02")]);
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn default_dir_is_under_codeaware_not_tmp() {
        assert_eq!(
            cache_dir_from_home("/home/u").unwrap(),
            PathBuf::from("/home/u/.codeaware/context-cache")
        );
        assert!(cache_dir_from_home("").is_err());
    }
}
