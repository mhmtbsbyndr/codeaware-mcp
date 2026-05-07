use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub goal: String,
    pub selected_paths: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub estimated_context_tokens: usize,
}

impl TraceEntry {
    pub fn new(task_id: String, goal: String) -> Self {
        Self {
            task_id,
            timestamp: Utc::now(),
            goal,
            selected_paths: Vec::new(),
            excluded_paths: Vec::new(),
            estimated_context_tokens: 0,
        }
    }
}
