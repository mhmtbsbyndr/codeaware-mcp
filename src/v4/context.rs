use serde::{Deserialize, Serialize};

use crate::v4::budget::BudgetState;
use crate::v4::context_items::{ContextItem, ExcludedContext};
use crate::v4::contracts::TaskContract;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub task_id: String,
    pub repo_root: String,
    pub contract: TaskContract,
    pub budget: BudgetState,
    pub selected_context: Vec<ContextItem>,
    pub excluded_context: Vec<ExcludedContext>,
    pub warnings: Vec<String>,
}

pub struct ContextAssembler;

impl ContextAssembler {
    pub fn default_excluded_paths() -> Vec<&'static str> {
        vec![
            ".git/**",
            "target/**",
            "node_modules/**",
            "vendor/**",
            "dist/**",
            "build/**",
            ".cache/**",
            ".codeaware/**",
        ]
    }
}
