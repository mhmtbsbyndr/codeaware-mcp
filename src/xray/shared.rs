//! Shared metrics file for communication between hook processes and xray server.
//! Hooks write to /tmp/codeaware-xray.json, xray server merges on each SSE tick.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const METRICS_FILE: &str = "/tmp/codeaware-xray.json";

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

fn metrics_path() -> PathBuf {
    PathBuf::from(METRICS_FILE)
}

/// Append a tool call to the shared metrics file (called from hooks).
pub fn append_tool_call(tool_name: &str, file_path: Option<&str>, raw_tokens: u64, compressed_tokens: u64) {
    let mut metrics = read_shared().unwrap_or_default();
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
    write_shared(&metrics);
}

/// Read shared metrics from file.
pub fn read_shared() -> Option<SharedMetrics> {
    let data = std::fs::read_to_string(metrics_path()).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write shared metrics to file (atomic via temp + rename).
fn write_shared(metrics: &SharedMetrics) {
    let json = match serde_json::to_string(metrics) {
        Ok(j) => j,
        Err(_) => return,
    };
    let tmp = format!("{}.tmp.{}", METRICS_FILE, std::process::id());
    if std::fs::write(&tmp, &json).is_ok() {
        let _ = std::fs::rename(&tmp, metrics_path());
    }
}

/// Reset shared metrics (called at session start).
pub fn reset_shared() {
    let _ = std::fs::remove_file(metrics_path());
}
