use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub timestamp: String,
    pub tool: String,
    pub file: Option<String>,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub duration_ms: u64,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditScoreEntry {
    pub file: String,
    pub symbol: String,
    pub score: u32,
    pub verdict: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub version: String,
    pub raw_tokens_total: u64,
    pub compressed_tokens_total: u64,
    pub tool_calls: u32,
    pub file_tokens: HashMap<String, u64>,
    pub edit_scores: Vec<EditScoreEntry>,
    pub timeline: Vec<TimelineEvent>,
    pub phase: String,
    pub session_id: String,
    pub error_loops: Vec<String>,
}

pub struct MetricsState {
    raw_tokens_total: u64,
    compressed_tokens_total: u64,
    tool_calls: u32,
    file_tokens: HashMap<String, u64>,
    edit_scores: Vec<EditScoreEntry>,
    timeline: Vec<TimelineEvent>,
    phase: String,
    session_id: String,
    error_loops: Vec<String>,
    compact_hint_40: bool,
    compact_hint_tokens: bool,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            raw_tokens_total: 0,
            compressed_tokens_total: 0,
            tool_calls: 0,
            file_tokens: HashMap::new(),
            edit_scores: Vec::new(),
            timeline: Vec::new(),
            phase: "Idle".to_string(),
            session_id: String::new(),
            error_loops: Vec::new(),
            compact_hint_40: false,
            compact_hint_tokens: false,
        }
    }

    pub fn record_tool_call(&mut self, _tool: &str, file: Option<&str>, raw_tokens: u64, compressed_tokens: u64) {
        self.tool_calls += 1;
        self.raw_tokens_total += raw_tokens;
        self.compressed_tokens_total += compressed_tokens;
        if let Some(f) = file {
            *self.file_tokens.entry(f.to_string()).or_insert(0) += raw_tokens;
        }
    }

    pub fn record_edit_score(&mut self, file: &str, symbol: &str, score: u32, verdict: &str) {
        self.edit_scores.push(EditScoreEntry {
            file: file.to_string(),
            symbol: symbol.to_string(),
            score,
            verdict: verdict.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn set_phase(&mut self, phase: &str) {
        self.phase = phase.to_string();
    }

    pub fn set_session_id(&mut self, id: &str) {
        self.session_id = id.to_string();
    }

    pub fn record_timeline_event(&mut self, tool: &str, file: Option<&str>, raw_tokens: u64, compressed_tokens: u64, duration_ms: u64, phase: &str) {
        self.timeline.push(TimelineEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool: tool.to_string(),
            file: file.map(|f| f.to_string()),
            raw_tokens,
            compressed_tokens,
            duration_ms,
            phase: phase.to_string(),
        });
    }

    /// Check and emit compaction hints when thresholds are exceeded.
    /// Each hint is only emitted once per session.
    pub fn check_compaction_hints(&mut self) {
        if !self.compact_hint_40 && self.tool_calls > 40 {
            self.compact_hint_40 = true;
            eprintln!("CodeAware: 40+ tool calls \u{2014} consider /compact to free context");
        }
        if !self.compact_hint_tokens && self.raw_tokens_total > 500_000 {
            self.compact_hint_tokens = true;
            eprintln!("CodeAware: 500k+ raw tokens \u{2014} consider /compact to free context");
        }
    }

    pub fn add_error_loop(&mut self, sig: &str) {
        if !self.error_loops.contains(&sig.to_string()) {
            self.error_loops.push(sig.to_string());
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            version: env!("CARGO_PKG_VERSION").to_string(),
            raw_tokens_total: self.raw_tokens_total,
            compressed_tokens_total: self.compressed_tokens_total,
            tool_calls: self.tool_calls,
            file_tokens: self.file_tokens.clone(),
            edit_scores: self.edit_scores.clone(),
            timeline: self.timeline.clone(),
            phase: self.phase.clone(),
            session_id: self.session_id.clone(),
            error_loops: self.error_loops.clone(),
        }
    }
}
