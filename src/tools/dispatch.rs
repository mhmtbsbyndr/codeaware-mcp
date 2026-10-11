use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::session::persistence::SessionDb;
use crate::session::state::SessionState;
use crate::tools::project_map::{project_map, ProjectMapInput};
use crate::tools::smart_edit::{smart_edit, EditPair, SmartEditInput};
use crate::tools::smart_read::{smart_read, smart_read_in_session, ReadMode, SmartReadInput};
use crate::tools::smart_run::{smart_run, SmartRunInput};
use crate::tools::validate_config::{validate_config, ValidateConfigInput};
use crate::xray::server::XrayServer;

/// Route a tool call to the appropriate handler and return the MCP result.
/// This keeps the dispatch logic separate from metrics/hooks in server.rs.
///
/// Every name advertised in `tools/list` (server.rs) must have an arm here.
/// Unknown names return a tool result with `isError: true`.
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

        // Project-root-aware tools: relative paths resolve against the session's project path.
        "project_map" => dispatch_project_map(tool_input, &project_root(state)),
        "smart_read" => dispatch_smart_read(tool_input, &project_root(state), state),
        "smart_edit" => dispatch_smart_edit(tool_input, &project_root(state)),
        "smart_run" => dispatch_smart_run(tool_input),
        "validate_config" => dispatch_validate_config(tool_input, &project_root(state)),
        "skill_lookup" => dispatch_skill_lookup(tool_input),

        _ => tool_error(&format!("Unknown tool: '{tool_name}' is not registered")),
    }
}

/// Error result per MCP tools/call convention: a normal result with `isError: true`.
fn tool_error(message: &str) -> Value {
    json!({
        "content": [{"type": "text", "text": message}],
        "isError": true
    })
}

fn project_root(state: &Arc<Mutex<SessionState>>) -> std::path::PathBuf {
    let path = state
        .lock()
        .map(|s| s.project_path().to_string())
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(path)
}

fn opt_str(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Accepts either a "a-b" string or an integer array `[a, b]` (schema form) and
/// returns the "a-b" string the handlers expect.
fn line_range_spec(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => {
            let nums: Vec<u64> = items.iter().filter_map(|v| v.as_u64()).collect();
            match nums.as_slice() {
                [a, b, ..] => Some(format!("{a}-{b}")),
                [a] => Some(format!("{a}-{a}")),
                [] => None,
            }
        }
        _ => None,
    }
}

fn dispatch_project_map(tool_input: &Value, project_root: &Path) -> Value {
    let input = ProjectMapInput {
        path: opt_str(tool_input, "path")
            .unwrap_or_else(|| project_root.to_string_lossy().to_string()),
        depth: tool_input.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize,
        include_symbols: tool_input
            .get("include_symbols")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        filter_language: opt_str(tool_input, "filter_language"),
        task_context: opt_str(tool_input, "task_context"),
    };
    match project_map(&input) {
        Ok(r) => {
            let tree: Vec<Value> = r
                .tree
                .iter()
                .map(|e| {
                    json!({
                        "path": e.path,
                        "loc": e.loc,
                        "access_frequency": e.access_frequency,
                        "symbols": e.symbols,
                        "last_modified": e.last_modified,
                    })
                })
                .collect();
            text_response(&json!({
                "project_name": r.project_name,
                "total_files": r.total_files,
                "total_loc": r.total_loc,
                "total_symbols": r.total_symbols,
                "intelligence_level": r.intelligence_level,
                "tree": tree,
                "dependencies": r.dependencies,
                "suggested_start": r.suggested_start,
            }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

fn dispatch_smart_read(
    tool_input: &Value,
    project_root: &Path,
    state: &Arc<Mutex<SessionState>>,
) -> Value {
    let Some(path) = opt_str(tool_input, "path") else {
        return tool_error("smart_read requires 'path'");
    };
    let mode_str = opt_str(tool_input, "mode").unwrap_or_else(|| "auto".to_string());
    let mode: ReadMode = match serde_json::from_value(Value::String(mode_str.clone())) {
        Ok(m) => m,
        Err(_) => {
            return tool_error(&format!(
                "smart_read: invalid mode '{mode_str}' (expected auto, skeleton, focused, full, diff)"
            ))
        }
    };
    let input = SmartReadInput {
        path,
        mode,
        focus: opt_str(tool_input, "focus"),
        lines: line_range_spec(tool_input.get("lines")),
        scope: opt_str(tool_input, "scope"),
    };
    let outcome = match state.lock() {
        Ok(mut s) => smart_read_in_session(&input, project_root, s.served_mut()),
        Err(_) => smart_read(&input, project_root),
    };
    match outcome {
        Ok(r) => text_response(&r),
        Err(e) => tool_error(&e.to_string()),
    }
}

fn dispatch_smart_edit(tool_input: &Value, project_root: &Path) -> Value {
    let Some(path) = opt_str(tool_input, "path") else {
        return tool_error("smart_edit requires 'path'");
    };
    let edits: Option<Vec<EditPair>> = match tool_input.get("edits") {
        None | Some(Value::Null) => None,
        Some(v) => match serde_json::from_value(v.clone()) {
            Ok(e) => Some(e),
            Err(e) => return tool_error(&format!("smart_edit: invalid 'edits': {e}")),
        },
    };
    let input = SmartEditInput {
        path,
        strategy: opt_str(tool_input, "strategy").unwrap_or_else(|| "text".to_string()),
        symbol: opt_str(tool_input, "symbol"),
        line_range: line_range_spec(tool_input.get("line_range")),
        edits,
        new_content: opt_str(tool_input, "new_content"),
        dry_run: tool_input
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        expected_hash: opt_str(tool_input, "expected_hash"),
    };
    match smart_edit(&input, project_root) {
        Ok(r) => text_response(&r),
        Err(e) => tool_error(&e.to_string()),
    }
}

/// Runs the async `smart_run` on a private current-thread runtime. The MCP server
/// is synchronous (main.rs reads stdin line by line) and has no runtime, so
/// `block_on` is safe here. It must not be called from inside an existing runtime.
fn dispatch_smart_run(tool_input: &Value) -> Value {
    let Some(command) = opt_str(tool_input, "command") else {
        return tool_error("smart_run requires 'command'");
    };
    let defaults = SmartRunInput::default();
    let input = SmartRunInput {
        command,
        max_output_lines: tool_input
            .get("max_output_lines")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(defaults.max_output_lines),
        capture_relevant_code: tool_input
            .get("capture_relevant_code")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.capture_relevant_code),
        scan_secrets: tool_input
            .get("scan_secrets")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.scan_secrets),
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return tool_error(&format!("smart_run: runtime error: {e}")),
    };
    match runtime.block_on(smart_run(&input)) {
        Ok(r) => text_response(&json!({
            "command": r.command,
            "command_type": r.command_type,
            "exit_code": r.exit_code,
            "duration_ms": r.duration_ms,
            "raw_lines": r.raw_lines,
            "compressed_lines": r.compressed_lines,
            "compression_ratio": r.compression_ratio,
            "summary": r.summary,
            "compressed_output": r.compressed_output,
            "failures": r.failures,
            "warnings": r.warnings,
            "secrets_detected": r.secrets_detected,
            "error_recurrence": r.error_recurrence,
            "suggested_next": r.suggested_next,
        })),
        Err(e) => tool_error(&e.to_string()),
    }
}

/// Skill excerpts are third-party text: each hit is returned inside an
/// `untrusted-skill` block (see `skills::untrusted_block`), never as plain instructions.
fn dispatch_skill_lookup(tool_input: &Value) -> Value {
    let Some(goal) = tool_input.get("goal").and_then(|g| g.as_str()) else {
        return tool_error("skill_lookup: 'goal' is required");
    };
    let max_results = tool_input
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(2)
        .clamp(1, 5) as usize;
    let max_bytes = tool_input
        .get("max_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(1536)
        .clamp(256, 8192) as usize;

    let hits: Vec<Value> = crate::skills::lookup(goal, max_results, max_bytes)
        .iter()
        .map(|hit| {
            json!({
                "id": hit.id,
                "source": hit.source,
                "license": hit.license,
                "score": hit.score,
                "block": crate::skills::untrusted_block(hit),
            })
        })
        .collect();

    text_response(&json!({ "hits": hits }))
}

fn dispatch_validate_config(tool_input: &Value, project_root: &Path) -> Value {
    let input = ValidateConfigInput {
        scope: opt_str(tool_input, "scope").unwrap_or_else(|| "all".to_string()),
    };
    match validate_config(&input, project_root) {
        Ok(r) => text_response(&r),
        Err(e) => tool_error(&e.to_string()),
    }
}

fn text_response<T: serde::Serialize>(value: &T) -> Value {
    json!({"content": [{"type": "text", "text": serde_json::to_string(value).unwrap_or_default()}]})
}

fn v4_tool_response<T: serde::Serialize, E: std::fmt::Display>(
    result: Result<T, E>,
) -> Value {
    match result {
        Ok(value) => {
            let envelope = Envelope::success(value, TrustLevel::Exact);
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
        Err(err) => {
            let envelope =
                Envelope::<()>::error(ErrorCode::EInternalError, false, Some(err.to_string()));
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
    }
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

    v4_tool_response(crate::v4::SemanticTools::find_symbol(req))
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

    v4_tool_response(crate::v4::SemanticTools::find_callers(req))
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

    v4_tool_response(crate::v4::SemanticTools::find_tests(req))
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

    v4_tool_response(crate::v4::SemanticTools::diff_impact(req))
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
