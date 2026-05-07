use serde::{Deserialize, Serialize};
use std::fs;

use crate::v4::budget::BudgetState;
use crate::v4::context::ContextPackage;
use crate::v4::context_items::{ContextItem, ContextItemKind, ExcludedContext};
use crate::v4::contracts::{TaskContract, TaskIntent, TaskScope};
use crate::v4::discovery::{CandidateDiscovery, DiscoveryConfig};
use crate::v4::index_builder::SemanticIndexBuilder;
use crate::v4::semantic_context::{SemanticContextAssembler, SemanticContextOptions};
use crate::v4::storage::V4Storage;
use crate::v4::summaries::SummaryGenerator;
use crate::v4::tokens::estimate_tokens;
use crate::v4::trace::TraceEntry;

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

        let mut selected_context = Vec::new();
        let mut files_read = 0usize;

        if let Ok(index) = SemanticIndexBuilder::build(&repo_root) {
            selected_context.extend(SemanticContextAssembler::assemble(
                &contract.goal,
                &index,
                SemanticContextOptions::default(),
            ));
        }

        if selected_context.is_empty() {
            let ranked_candidates = CandidateDiscovery::discover_ranked(
                &repo_root,
                &contract.goal,
                DiscoveryConfig::default(),
            )
            .unwrap_or_default();

            for candidate in ranked_candidates.into_iter().take(contract.scope.max_files_read) {
                let full_path = std::path::Path::new(&repo_root).join(&candidate.path);
                let Ok(content) = fs::read_to_string(&full_path) else {
                    continue;
                };

                let summary = SummaryGenerator::summarize_file(candidate.path.clone(), &content);
                files_read += 1;

                selected_context.push(ContextItem {
                    kind: ContextItemKind::FileSummary,
                    path: Some(summary.path),
                    symbol: None,
                    content: summary.summary,
                    reason: candidate.reason,
                    estimated_tokens: summary.estimated_tokens,
                });

                let current_tokens: usize = selected_context
                    .iter()
                    .map(|item| item.estimated_tokens)
                    .sum();

                if current_tokens >= contract.scope.max_context_tokens {
                    break;
                }
            }
        }

        if selected_context.is_empty() {
            let summary = SummaryGenerator::summarize_file(
                "contract://v4",
                "CodeAware v4 task contract active. Use bounded context only.",
            );
            selected_context.push(ContextItem {
                kind: ContextItemKind::Contract,
                path: Some(summary.path.clone()),
                symbol: None,
                content: summary.summary.clone(),
                reason: "Fallback contract context. No readable candidates selected.".to_string(),
                estimated_tokens: summary.estimated_tokens,
            });
        }

        let estimated_context_tokens: usize = selected_context
            .iter()
            .map(|item| item.estimated_tokens.max(estimate_tokens(&item.content)))
            .sum();

        let budget = BudgetState {
            task_id: contract.task_id.clone(),
            files_read,
            files_changed: 0,
            tool_calls: 1,
            estimated_context_tokens,
            max_files_read: contract.scope.max_files_read,
            max_files_changed: contract.scope.max_files_changed,
            max_tool_calls: contract.scope.max_tool_calls,
            max_context_tokens: contract.scope.max_context_tokens,
        };

        let excluded_context: Vec<ExcludedContext> = contract
            .forbidden_paths
            .iter()
            .map(|path| ExcludedContext {
                path: path.clone(),
                reason: "Forbidden by default v4 task contract.".to_string(),
            })
            .collect();

        let context_package = ContextPackage {
            task_id: contract.task_id.clone(),
            repo_root: repo_root.clone(),
            contract: contract.clone(),
            budget,
            selected_context,
            excluded_context,
            warnings: vec![
                "Semantic-first context assembly active.".to_string(),
                "Falls back to summary-first file context when no semantic items are found.".to_string(),
            ],
        };

        let mut trace = TraceEntry::new(contract.task_id.clone(), contract.goal.clone());
        trace.selected_paths = context_package
            .selected_context
            .iter()
            .filter_map(|item| item.path.clone())
            .collect();
        trace.excluded_paths = context_package
            .excluded_context
            .iter()
            .map(|item| item.path.clone())
            .collect();
        trace.estimated_context_tokens = context_package.budget.estimated_context_tokens;

        if let Ok(line) = serde_json::to_string(&trace) {
            let storage = V4Storage::new(&repo_root);
            let _ = storage.append_jsonl("traces/task_traces.jsonl", &line);
        }

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
            "Prefer semantic symbols/imports/calls/tests over raw file reads.".to_string(),
            "Stop after one implementation step and show the diff.".to_string(),
        ]
    }
}
