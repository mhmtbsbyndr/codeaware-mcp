use serde::{Deserialize, Serialize};

use crate::v4::budget::BudgetState;
use crate::v4::cochange::CoChangeHistory;
use crate::v4::context::ContextPackage;
use crate::v4::context_items::{ContextItem, ContextItemKind, ExcludedContext};
use crate::v4::contracts::{TaskContract, TaskIntent, TaskScope};
use crate::v4::discovery::{CandidateDiscovery, DiscoveryConfig};
use crate::v4::index_builder::SemanticIndexBuilder;
use crate::v4::query::normalize_goal;
use crate::v4::semantic_context::{
    rank_candidates, FileCandidate, SemanticContextAssembler, SemanticContextOptions, CANDIDATE_LIMIT,
};
use crate::v4::storage::V4Storage;
use crate::v4::summaries::{read_capped, SummaryGenerator};
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

/// Shape of the file selection returned by `get_task_context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextOutput {
    /// Symbols, imports, calls and tests from the semantic assembler (one line each).
    #[default]
    Full,
    /// Ranked file candidates: path, score and a reason of at most 8 words. No bodies.
    Candidates,
}

/// Output used by `get_task_context`. Candidates: recall@5 equal to the full output and recall@10
/// higher, at about 1/13 of the tokens (benches/RESULTS.md, candidate section).
pub const DEFAULT_CONTEXT_OUTPUT: ContextOutput = ContextOutput::Candidates;

/// Candidate items become context items with the file path, the reason as content and the
/// score in the reason field, so the response envelope stays the same for both outputs.
fn candidate_item(candidate: FileCandidate) -> ContextItem {
    let content = candidate.reason.clone();
    ContextItem {
        kind: ContextItemKind::FileExcerpt,
        path: Some(candidate.path),
        symbol: None,
        estimated_tokens: estimate_tokens(&content),
        content,
        reason: format!("Ranked candidate, score {:.3}.", candidate.score),
    }
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
        Self::get_task_context_with(req, repo_root, DEFAULT_CONTEXT_OUTPUT, None)
    }

    /// `get_task_context` with an explicit output shape. `history` is only used by
    /// `ContextOutput::Candidates`: `Some` fuses the BM25 list with co-change partners.
    pub fn get_task_context_with(
        req: GetTaskContextRequest,
        repo_root: String,
        output: ContextOutput,
        history: Option<&CoChangeHistory>,
    ) -> GetTaskContextResponse {
        let mut contract = Self::default_contract(req.goal.clone(), req.intent);
        if let Some(task_id) = req.task_id {
            contract.task_id = task_id;
        }

        let mut selected_context = Vec::new();
        let mut files_read = 0usize;

        if let Ok(index) = SemanticIndexBuilder::build(&repo_root) {
            match output {
                ContextOutput::Full => selected_context.extend(SemanticContextAssembler::assemble(
                    &contract.goal,
                    &index,
                    SemanticContextOptions::default(),
                )),
                ContextOutput::Candidates => selected_context.extend(
                    rank_candidates(&contract.goal, &index, history, CANDIDATE_LIMIT)
                        .into_iter()
                        .map(candidate_item),
                ),
            }
        }

        // Summary fallback: used only when no goal-relevant semantic item was found. The
        // contract keeps the original goal; only the retrieval query is normalized.
        let retrieval_query = normalize_goal(&contract.goal).search_query();
        if selected_context.is_empty() {
            let ranked_candidates = CandidateDiscovery::discover_ranked(
                &repo_root,
                &retrieval_query,
                DiscoveryConfig::default(),
            )
            .unwrap_or_default();

            for candidate in ranked_candidates.into_iter().take(contract.scope.max_files_read) {
                let full_path = std::path::Path::new(&repo_root).join(&candidate.path);
                let Some(content) = read_capped(&full_path) else {
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
                match output {
                    ContextOutput::Full => "Semantic-first context assembly active.".to_string(),
                    ContextOutput::Candidates => {
                        "Ranked candidate list active: path, score and reason only, no file bodies.".to_string()
                    }
                },
                "Falls back to summary-first file context when no semantic items are found.".to_string(),
                format!("Retrieval terms from normalized goal: {retrieval_query}"),
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
            if let Err(err) = storage.append_jsonl("traces/task_traces.jsonl", &line) {
                eprintln!("codeaware v4: failed to append task trace: {err}");
            }
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
