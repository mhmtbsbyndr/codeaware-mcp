//! Hook spool: durable hand-off of hook events to a later drain step.
//!
//! A short-lived hook writes one JSON file per event into the spool directory
//! and returns. Pending events are processed on the next hook call or on
//! session stop, so an event survives if the hook process dies in between.
//!
//! Crash safety:
//! - Enqueue writes `<name>.tmp` and renames it to `<name>.json`. A reader
//!   never sees a partially written event.
//! - Drain claims an event by renaming it to `<name>.json.claimed`. Only one
//!   process can win the rename, so an event is never processed twice in
//!   parallel.
//! - A failed handler renames the claim back to `.json` so it is retried.
//! - Claims older than `CLAIM_TTL` (left behind by a crashed drain) are
//!   returned to the pending set.

use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SPOOL_DIR_NAME: &str = "hook-spool";
const JSON_EXT: &str = ".json";
const CLAIMED_EXT: &str = ".json.claimed";
const CLAIM_TTL: Duration = Duration::from_secs(60);

/// Event kinds accepted by the spool.
pub const KINDS: &[&str] = &["observation", "file_edit", "session_stop"];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DrainReport {
    pub processed: usize,
    pub failed: usize,
    pub skipped_invalid: usize,
}

/// Spool directory below `home`: `<home>/.codeaware/hook-spool`.
pub fn spool_dir_from_home(home: &str) -> Result<PathBuf, String> {
    if home.trim().is_empty() {
        return Err("HOME is not set; cannot determine the hook spool directory".into());
    }
    Ok(PathBuf::from(home).join(".codeaware").join(SPOOL_DIR_NAME))
}

fn default_dir() -> Result<PathBuf, String> {
    spool_dir_from_home(&std::env::var("HOME").unwrap_or_default())
}

/// Enqueue an event in the default spool directory.
pub fn enqueue(kind: &str, payload: &Value) -> Result<PathBuf, String> {
    enqueue_in(&default_dir()?, kind, payload)
}

/// Drain the default spool directory.
pub fn drain<F>(handler: F) -> DrainReport
where
    F: FnMut(&str, &Value) -> Result<(), String>,
{
    match default_dir() {
        Ok(dir) => drain_in(&dir, handler),
        Err(_) => DrainReport::default(),
    }
}

/// Write one event atomically into `dir` and return its final path.
pub fn enqueue_in(dir: &Path, kind: &str, payload: &Value) -> Result<PathBuf, String> {
    todo!("stub")
}

/// Process pending events in `dir`, oldest first.
pub fn drain_in<F>(dir: &Path, mut handler: F) -> DrainReport
where
    F: FnMut(&str, &Value) -> Result<(), String>,
{
    todo!("stub")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "codeaware-spool-test-{}-{}",
            name,
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn enqueue_then_drain_runs_handler_once_and_removes_event() {
        let dir = tmp_dir("basic");
        let path = enqueue_in(&dir, "observation", &json!({"tool_name": "smart_read"})).unwrap();
        assert!(path.exists());

        let mut seen = Vec::new();
        let report = drain_in(&dir, |kind, payload| {
            seen.push((kind.to_string(), payload["tool_name"].as_str().unwrap().to_string()));
            Ok(())
        });
        assert_eq!(report, DrainReport { processed: 1, failed: 0, skipped_invalid: 0 });
        assert_eq!(seen, vec![("observation".to_string(), "smart_read".to_string())]);
        assert!(!path.exists());

        let again = drain_in(&dir, |_, _| Ok(()));
        assert_eq!(again.processed, 0);
    }

    #[test]
    fn drain_processes_in_enqueue_order() {
        let dir = tmp_dir("order");
        for i in 0..5 {
            enqueue_in(&dir, "file_edit", &json!({"n": i})).unwrap();
        }
        let mut order = Vec::new();
        drain_in(&dir, |_, p| {
            order.push(p["n"].as_i64().unwrap());
            Ok(())
        });
        assert_eq!(order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn failed_handler_keeps_event_for_retry() {
        let dir = tmp_dir("retry");
        let path = enqueue_in(&dir, "session_stop", &json!({"session_id": "s1"})).unwrap();
        let first = drain_in(&dir, |_, _| Err("db locked".into()));
        assert_eq!(first.failed, 1);
        assert!(path.exists(), "event must survive a failed handler");

        let second = drain_in(&dir, |_, _| Ok(()));
        assert_eq!(second.processed, 1);
        assert!(!path.exists());
    }

    #[test]
    fn partial_tmp_file_is_ignored_and_corrupt_event_is_quarantined() {
        let dir = tmp_dir("partial");
        fs::write(dir.join(".half-written.tmp"), b"{\"kind\":").unwrap();
        fs::write(dir.join("00000000000000000001-bad.json"), b"not json").unwrap();
        enqueue_in(&dir, "observation", &json!({"ok": true})).unwrap();

        let mut calls = 0;
        let report = drain_in(&dir, |_, _| {
            calls += 1;
            Ok(())
        });
        assert_eq!(calls, 1);
        assert_eq!(report.processed, 1);
        assert_eq!(report.skipped_invalid, 1);
        assert!(dir.join(".half-written.tmp").exists());
    }

    #[test]
    fn stale_claim_from_crashed_drain_is_recovered() {
        let dir = tmp_dir("stale-claim");
        let path = enqueue_in(&dir, "observation", &json!({"x": 1})).unwrap();
        let claimed = PathBuf::from(format!("{}.claimed", path.display()));
        fs::rename(&path, &claimed).unwrap();
        // Make the claim look old.
        let old = SystemTime::now() - Duration::from_secs(3600);
        File::options().write(true).open(&claimed).unwrap().set_modified(old).unwrap();

        let report = drain_in(&dir, |_, _| Ok(()));
        assert_eq!(report.processed, 1);
        assert!(!claimed.exists());
    }

    #[test]
    fn unknown_kind_is_rejected_on_enqueue() {
        let dir = tmp_dir("kind");
        assert!(enqueue_in(&dir, "../escape", &json!({})).is_err());
        assert!(enqueue_in(&dir, "nope", &json!({})).is_err());
    }

    #[test]
    fn spool_dir_is_under_codeaware_and_rejects_empty_home() {
        assert_eq!(
            spool_dir_from_home("/home/u").unwrap(),
            PathBuf::from("/home/u/.codeaware/hook-spool")
        );
        assert!(spool_dir_from_home("  ").is_err());
    }
}
