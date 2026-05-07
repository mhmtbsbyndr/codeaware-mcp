use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContract {
    pub task_id: String,
    pub intent: TaskIntent,
    pub goal: String,
    pub scope: TaskScope,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub stop_conditions: Vec<StopCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskScope {
    pub max_files_read: usize,
    pub max_files_changed: usize,
    pub max_tool_calls: usize,
    pub max_context_tokens: usize,
    pub max_output_tokens: Option<usize>,
}

impl Default for TaskScope {
    fn default() -> Self {
        Self {
            max_files_read: 8,
            max_files_changed: 4,
            max_tool_calls: 12,
            max_context_tokens: 30_000,
            max_output_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskIntent {
    Analyze,
    ImplementFeature,
    FixBug,
    Refactor,
    WriteTests,
    UpdateDocs,
    Review,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StopCondition {
    ShowDiff,
    WaitForHuman,
    BudgetExceeded,
    ContractViolation,
    TestsRequired,
}
