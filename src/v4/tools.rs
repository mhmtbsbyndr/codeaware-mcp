use serde::{Deserialize, Serialize};

use crate::v4::contracts::{TaskContract, TaskIntent, TaskScope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskContractRequest {
    pub goal: String,
    pub intent: TaskIntent,
    pub allowed_paths: Option<Vec<String>>,
    pub forbidden_paths: Option<Vec<String>>,
    pub scope: Option<TaskScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckBudgetRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskContextRequest {
    pub task_id: Option<String>,
    pub goal: String,
    pub intent: TaskIntent,
}

pub struct V4Tools;

impl V4Tools {
    pub fn default_contract(goal: String, intent: TaskIntent) -> TaskContract {
        TaskContract {
            task_id: uuid::Uuid::new_v4().to_string(),
            intent,
            goal,
            scope: TaskScope::default(),
            allowed_paths: vec!["src/**".to_string()],
            forbidden_paths: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                "vendor/**".to_string(),
            ],
            stop_conditions: vec![
                crate::v4::contracts::StopCondition::ShowDiff,
                crate::v4::contracts::StopCondition::WaitForHuman,
            ],
        }
    }
}
