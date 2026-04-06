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
