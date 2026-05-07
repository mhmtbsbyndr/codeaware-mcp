use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySnapshot {
    pub task_id: String,
    pub selected_paths: Vec<String>,
    pub semantic_symbols: Vec<String>,
    pub estimated_context_tokens: usize,
}

pub struct SemanticRecovery;

impl SemanticRecovery {
    pub fn compact_summary(snapshot: &RecoverySnapshot) -> String {
        format!(
            "Task {} used {} semantic symbols across {} paths with ~{} tokens.",
            snapshot.task_id,
            snapshot.semantic_symbols.len(),
            snapshot.selected_paths.len(),
            snapshot.estimated_context_tokens
        )
    }
}
