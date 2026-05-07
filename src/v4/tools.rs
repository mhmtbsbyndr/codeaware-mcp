use serde::{Deserialize, Serialize};

use crate::v4::budget::BudgetState;
use crate::v4::context::ContextPackage;
use crate::v4::context_items::{ContextItem, ContextItemKind, ExcludedContext};
use crate::v4::contracts::{TaskContract, TaskIntent, TaskScope};
use crate::v4::summaries::SummaryGenerator;
use crate::v4::tokens::estimate_tokens;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskContextResponse {
    pub task_id: String,
    pub context_package: ContextPackage,
    pub agent_instructions: Vec<String>,
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
                ".git/**".to_string(),
                ".codeaware/**".to_string(),
            ],
            stop_conditions: vec![
                crate::v4::contracts::StopCondition::ShowDiff,
                crate::v4::contracts::StopCondition::WaitForHuman,
            ],
        }
    }

    pub fn get_task_context(req: GetTaskContextRequest, repo_root: String) -> GetTaskContextResponse {
        let mut contract = Self::default_contract(req.goal.clone(), req.intent);
        if let Some(task_id) = req.task_id {
            contract.task_id = task_id;
        }

        let summary = SummaryGenerator::summarize_file(
            "contract://v4",
            "CodeAware v4 task contract active. Use bounded context only.",
        );

        let selected_context = vec![ContextItem {
            kind: ContextItemKind::Contract,
            path: Some(summary.path.clone()),
            symbol: None,
            content: summary.summary.clone(),
            reason: "Every v4 task starts with a contract before repository exploration.".to_string(),
            estimated_tokens: summary.estimated_tokens,
        }];

        let estimated_context_tokens: usize = selected_context
            .iter()
            .map(|item| estimate_tokens(&item.content))
            .sum();

        let budget = BudgetState {
            task_id: contract.task_id.clone(),
            files_read: 0,
            files_changed: 0,
            tool_calls: 1,
            estimated_context_tokens,
            max_files_read: contract.scope.max_files_read,
            max_files_changed: contract.scope.max_files_changed,
            max_tool_calls: contract.scope.max_tool_calls,
            max_context_tokens: contract.scope.max_context_tokens,
        };

        let excluded_context = contract
            .forbidden_paths
            .iter()
            .map(|path| ExcludedContext {
                path: path.clone(),
                reason: "Forbidden by default v4 task contract.".to_string(),
            })
            .collect();

        let context_package = ContextPackage {
            task_id: contract.task_id.clone(),
            repo_root,
            contract: contract.clone(),
            budget,
            selected_context,
            excluded_context,
            warnings: vec![
                "Phase 1 context assembly is conservative and contract-first.".to_string(),
                "Summary-first context generation active.".to_string(),
            ],
        };

        GetTaskContextResponse {
            task_id: contract.task_id,
            context_package,
            agent_instructions: Self::agent_instructions(),
        }
    }

    pub fn agent_instructions() -> Vec<String> {
        vec![
            "You are operating under a CodeAware v4 task contract.".to_string(),
            "Do not scan the full repository.".to_string(),
            "Do not open files outside allowed paths unless a new contract is created.".to_string(),
            "Do not re-read files already summarized in the context package.".to_string(),
            "Stop after one implementation step and show the diff.".to_string(),
        ]
    }
}
