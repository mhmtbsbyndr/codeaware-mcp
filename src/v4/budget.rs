use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetState {
    pub task_id: String,
    pub files_read: usize,
    pub files_changed: usize,
    pub tool_calls: usize,
    pub estimated_context_tokens: usize,
    pub max_files_read: usize,
    pub max_files_changed: usize,
    pub max_tool_calls: usize,
    pub max_context_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetRemaining {
    pub files_read: isize,
    pub files_changed: isize,
    pub tool_calls: isize,
    pub context_tokens: isize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheck {
    pub ok: bool,
    pub remaining: BudgetRemaining,
    pub warnings: Vec<String>,
}

impl BudgetState {
    pub fn check(&self) -> BudgetCheck {
        let remaining = BudgetRemaining {
            files_read: self.max_files_read as isize - self.files_read as isize,
            files_changed: self.max_files_changed as isize - self.files_changed as isize,
            tool_calls: self.max_tool_calls as isize - self.tool_calls as isize,
            context_tokens: self.max_context_tokens as isize
                - self.estimated_context_tokens as isize,
        };

        let ok = remaining.files_read >= 0
            && remaining.files_changed >= 0
            && remaining.tool_calls >= 0
            && remaining.context_tokens >= 0;

        let mut warnings = Vec::new();

        if remaining.context_tokens < 5_000 {
            warnings.push("Context token budget nearly exhausted".to_string());
        }

        BudgetCheck {
            ok,
            remaining,
            warnings,
        }
    }
}
