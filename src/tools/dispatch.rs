use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::session::persistence::SessionDb;
use crate::session::state::SessionState;
use crate::xray::server::XrayServer;

/// Route a tool call to the appropriate handler and return the MCP result.
/// This keeps the dispatch logic separate from metrics/hooks in server.rs.
pub fn dispatch_tool(
    tool_name: &str,
    tool_input: &Value,
    state: &Arc<Mutex<SessionState>>,
    db: &Option<Arc<Mutex<SessionDb>>>,
    xray_server: &Mutex<Option<XrayServer>>,
    metrics: &Arc<Mutex<crate::xray::metrics::MetricsState>>,
) -> Value {
    match tool_name {
        // CodeAware v4 semantic runtime tools
        "codeaware.get_task_context" => dispatch_v4_get_task_context(tool_input),
        "codeaware.find_symbol" => dispatch_v4_find_symbol(tool_input),
        "codeaware.find_callers" => dispatch_v4_find_callers(tool_input),
        "codeaware.find_tests" => dispatch_v4_find_tests(tool_input),
        "codeaware.diff_impact" => dispatch_v4_diff_impact(tool_input),

        // Foundation runtime tools
        "token_stats" => crate::tools::foundation::handle_token_stats(tool_input),
        "token_savings_report" => {
            crate::tools::foundation::handle_token_savings_report(tool_input)
        }
        "benchmark_compression" => {
            crate::tools::foundation::handle_benchmark_compression(tool_input)
        }
        "provide_feedback" => crate::tools::foundation::handle_provide_feedback(tool_input),
        "token_quality" => crate::tools::foundation::handle_token_quality(tool_input),

        // Context-window optimizer tools
        "get_relevant_code" => crate::tools::foundation::handle_get_relevant_code(tool_input),
        "code_search" => crate::tools::foundation::handle_code_search(tool_input),
        "get_relevant_test_errors" => {
            crate::tools::foundation::handle_get_relevant_test_errors(tool_input)
        }
        "get_project_context" => crate::tools::foundation::handle_get_project_context(tool_input),
        "tool_manager" => crate::tools::foundation::handle_tool_manager(tool_input),

        "workspace_state" => {
            let result = crate::tools::workspace_state::handle_workspace_state(tool_input, state);
            json!({
                "content": [{
                    "type": "text",
                    "text": result.to_string()
                }]
            })
        }
        "xray" => {
            match crate::tools::xray::handle_xray(Arc::clone(metrics), xray_server) {
                Ok(result) => {
                    let envelope = Envelope::success(result, TrustLevel::Exact);
                    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
                }
                Err(e) => {
                    let envelope =
                        Envelope::<()>::error(ErrorCode::EInternalError, false, Some(e));
                    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
                }
            }
        }
        "save_memory" | "search_memory" | "memory_timeline" | "summarize_memory" => {
            dispatch_memory_tool(tool_name, tool_input, db)
        }
        "git_diff" => crate::tools::git_intelligence::handle_git_diff(tool_input),
        "git_blame" => crate::tools::git_intelligence::handle_git_blame(tool_input),
        "git_changelog" => crate::tools::git_intelligence::handle_git_changelog(tool_input),
        "smart_refactor" => crate::tools::smart_refactor::handle_smart_refactor(tool_input),
        "test_coverage_map" => {
            crate::tools::test_coverage_map::handle_test_coverage_map(tool_input)
        }
        _ => json!({
            "content": [{
                "type": "text",
                "text": "Tool not yet implemented"
            }]
        }),
    }
}

fn text_response<T: serde::Serialize>(value: &T) -> Value {
    json!({"content": [{"type": "text", "text": serde_json::to_string(value).unwrap_or_default()}]})
}

fn dispatch_v4_get_task_context(tool_input: &Value) -> Value {
    let repo_root = tool_input
        .get("repo_root")
        .and_then(|value| value.as_str())
        .unwrap_or(".")
        .to_string();

    let goal = tool_input
        .get("goal")
        .and_then(|value| value.as_str())
        .unwrap_or("Build bounded semantic context")
        .to_string();

    let req = crate::v4::GetTaskContextRequest {
        task_id: tool_input
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string()),
        goal,
        intent: crate::v4::TaskIntent::Unknown,
    };

    text_response(&crate::v4::V4Tools::get_task_context(req, repo_root))
}

fn dispatch_v4_find_symbol(tool_input: &Value) -> Value {
    let req = crate::v4::FindSymbolRequest {
        repo_root: tool_input
            .get("repo_root")
            .and_then(|value| value.as_str())
            .unwrap_or(".")
            .to_string(),
        query: tool_input
            .get("query")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    };

    text_response(&crate::v4::SemanticTools::find_symbol(req))
}

fn dispatch_v4_find_callers(tool_input: &Value) -> Value {
    let req = crate::v4::FindCallersRequest {
        repo_root: tool_input
            .get("repo_root")
            .and_then(|value| value.as_str())
            .unwrap_or(".")
            .to_string(),
        symbol: tool_input
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    };

    text_response(&crate::v4::SemanticTools::find_callers(req))
}

fn dispatch_v4_find_tests(tool_input: &Value) -> Value {
    let req = crate::v4::FindTestsRequest {
        repo_root: tool_input
            .get("repo_root")
            .and_then(|value| value.as_str())
            .unwrap_or(".")
            .to_string(),
        symbol: tool_input
            .get("symbol")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    };

    text_response(&crate::v4::SemanticTools::find_tests(req))
}

fn dispatch_v4_diff_impact(tool_input: &Value) -> Value {
    let req = crate::v4::DiffImpactRequest {
        repo_root: tool_input
            .get("repo_root")
            .and_then(|value| value.as_str())
            .unwrap_or(".")
            .to_string(),
        changed_path: tool_input
            .get("changed_path")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    };

    text_response(&crate::v4::SemanticTools::diff_impact(req))
}

fn dispatch_memory_tool(
    tool_name: &str,
    tool_input: &Value,
    db: &Option<Arc<Mutex<SessionDb>>>,
) -> Value {
    let db_arc = match db {
        Some(db) => db,
        None => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some("Memory database unavailable".to_string()),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };
    let db_guard = match db_arc.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            eprintln!("Warning: memory database mutex poisoned, recovering");
            let guard = poisoned.into_inner();
            if let Err(e) = guard.rollback() {
                eprintln!("Warning: ROLLBACK after mutex recovery: {e}");
            }
            guard
        }
    };
    let result = match tool_name {
        "save_memory" => crate::tools::memory::handle_save_memory(tool_input, &db_guard),
        "search_memory" => crate::tools::memory::handle_search_memory(tool_input, &db_guard),
        "memory_timeline" => crate::tools::memory::handle_memory_timeline(tool_input, &db_guard),
        "summarize_memory" => {
            crate::tools::memory_summary::handle_summarize_memory(tool_input, &db_guard)
        }
        _ => unreachable!(),
    };
    json!({"content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}]})
}
