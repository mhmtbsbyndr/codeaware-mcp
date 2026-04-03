use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use crate::session::state::SessionState;
use crate::session::persistence::SessionDb;
use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;
use crate::envelope::{Envelope, ErrorCode, TrustLevel};

pub struct McpServer {
    state: Arc<Mutex<SessionState>>,
    metrics: Arc<Mutex<MetricsState>>,
    xray_server: Mutex<Option<XrayServer>>,
    db: Option<Arc<Mutex<SessionDb>>>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        let metrics = Arc::new(Mutex::new(MetricsState::new()));

        // Auto-start XRay dashboard on server init
        let xray = XrayServer::start(Arc::clone(&metrics)).ok();
        if let Some(ref srv) = xray {
            eprintln!("XRay dashboard: {}", srv.url());
        }

        let db = Self::open_db();

        McpServer {
            state: Arc::new(Mutex::new(SessionState::new("."))),
            metrics,
            xray_server: Mutex::new(xray),
            db,
        }
    }

    pub fn new_with_state(state: Arc<Mutex<SessionState>>) -> Self {
        let metrics = Arc::new(Mutex::new(MetricsState::new()));

        let xray = XrayServer::start(Arc::clone(&metrics)).ok();
        if let Some(ref srv) = xray {
            eprintln!("XRay dashboard: {}", srv.url());
        }

        let db = Self::open_db();

        McpServer {
            state,
            metrics,
            xray_server: Mutex::new(xray),
            db,
        }
    }

    fn open_db() -> Option<Arc<Mutex<SessionDb>>> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = std::path::PathBuf::from(home)
            .join(".codeaware")
            .join("session.db");
        match SessionDb::open(&db_path) {
            Ok(db) => Some(Arc::new(Mutex::new(db))),
            Err(e) => {
                eprintln!("Warning: memory database unavailable: {e}");
                None
            }
        }
    }

    pub fn handle_message(&self, message: &str) -> Option<String> {
        let parsed: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(_) => return Some(self.respond_error(Value::Null, -32700, "Parse error")),
        };

        let method = match parsed.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                let id = parsed.get("id").cloned().unwrap_or(Value::Null);
                return Some(self.respond_error(id, -32600, "Invalid Request: missing method"));
            }
        };

        // Notifications have no id and expect no response
        if method.starts_with("notifications/") {
            return None;
        }

        let id = match parsed.get("id").cloned() {
            Some(id) => id,
            None => return Some(self.respond_error(Value::Null, -32600, "Invalid Request: missing id")),
        };

        match method {
            "initialize" => Some(self.respond(id, self.handle_initialize())),
            "tools/list" => Some(self.respond(id, self.handle_tools_list())),
            "tools/call" => Some(self.respond(id, self.handle_tools_call(&parsed))),
            _ => Some(self.respond_error(id, -32601, "Method not found")),
        }
    }

    fn handle_initialize(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "codeaware",
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }

    fn handle_tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "project_map",
                    "description": "Generate a structural map of the project directory",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Root path to map"
                            },
                            "depth": {
                                "type": "integer",
                                "description": "Maximum directory depth"
                            },
                            "include_symbols": {
                                "type": "boolean",
                                "description": "Include symbol definitions"
                            },
                            "filter_language": {
                                "type": "string",
                                "description": "Filter by programming language"
                            },
                            "task_context": {
                                "type": "string",
                                "description": "Context hint for the task"
                            }
                        }
                    }
                },
                {
                    "name": "smart_read",
                    "description": "Read a file with intelligent focus and context extraction",
                    "inputSchema": {
                        "type": "object",
                        "required": ["path"],
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to read"
                            },
                            "mode": {
                                "type": "string",
                                "description": "Read mode (full, summary, symbol)"
                            },
                            "focus": {
                                "type": "string",
                                "description": "Focus area or symbol name"
                            },
                            "lines": {
                                "type": "array",
                                "items": { "type": "integer" },
                                "description": "Specific line numbers to read"
                            },
                            "scope": {
                                "type": "string",
                                "description": "Scope of reading context"
                            }
                        }
                    }
                },
                {
                    "name": "smart_edit",
                    "description": "Edit a file using intelligent strategies",
                    "inputSchema": {
                        "type": "object",
                        "required": ["path"],
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path to edit"
                            },
                            "strategy": {
                                "type": "string",
                                "description": "Edit strategy (symbol, line_range, full)"
                            },
                            "symbol": {
                                "type": "string",
                                "description": "Symbol to edit"
                            },
                            "line_range": {
                                "type": "array",
                                "items": { "type": "integer" },
                                "description": "Line range [start, end]"
                            },
                            "edits": {
                                "type": "array",
                                "description": "Array of edit operations",
                                "items": {
                                    "type": "object"
                                }
                            },
                            "new_content": {
                                "type": "string",
                                "description": "New content to write"
                            },
                            "dry_run": {
                                "type": "boolean",
                                "description": "Preview changes without applying"
                            },
                            "expected_hash": {
                                "type": "string",
                                "description": "Expected hash for conflict detection"
                            }
                        }
                    }
                },
                {
                    "name": "smart_run",
                    "description": "Execute a command with intelligent output capture",
                    "inputSchema": {
                        "type": "object",
                        "required": ["command"],
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "Command to execute"
                            },
                            "max_output_lines": {
                                "type": "integer",
                                "description": "Maximum number of output lines to capture"
                            },
                            "capture_relevant_code": {
                                "type": "boolean",
                                "description": "Include relevant code context in output"
                            },
                            "scan_secrets": {
                                "type": "boolean",
                                "description": "Scan output for accidental secret exposure"
                            }
                        }
                    }
                },
                {
                    "name": "session_status",
                    "description": "Get the current session and workspace status",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": {
                                "type": "string",
                                "description": "Scope of the status report"
                            },
                            "include_verification": {
                                "type": "boolean",
                                "description": "Include verification checks"
                            }
                        }
                    }
                },
                {
                    "name": "workspace_state",
                    "description": "Manage typed workspace state slots (recent_targets, error_signatures, co_access_candidates, verification_state, active_task)",
                    "inputSchema": {
                        "type": "object",
                        "required": ["action", "slot"],
                        "properties": {
                            "action": {
                                "type": "string",
                                "enum": ["read", "write", "clear"],
                                "description": "Action to perform"
                            },
                            "slot": {
                                "type": "string",
                                "enum": [
                                    "recent_targets",
                                    "error_signatures",
                                    "co_access_candidates",
                                    "verification_state",
                                    "active_task"
                                ],
                                "description": "Slot name"
                            },
                            "value": {
                                "type": "object",
                                "description": "Only for action=write. Typed per slot."
                            }
                        }
                    }
                },
                {
                    "name": "validate_config",
                    "description": "Validate CodeAware configuration files",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": {
                                "type": "string",
                                "description": "Scope of validation (local, global, all)"
                            }
                        }
                    }
                },
                {
                    "name": "xray",
                    "description": "Open a live browser dashboard showing token consumption, compression savings, file heatmap, edit confidence scores, and session metrics in real-time",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "save_memory",
                    "description": "Save a semantic observation to persistent memory. Memories survive across sessions and are searchable via FTS5 full-text search.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["text"],
                        "properties": {
                            "text": { "type": "string", "description": "The observation text" },
                            "title": { "type": "string", "description": "Short title" },
                            "type": { "type": "string", "enum": ["bugfix","feature","refactor","change","discovery","decision"], "description": "Observation type (default: discovery)" },
                            "concepts": { "type": "array", "items": { "type": "string" }, "description": "Concept tags: how-it-works, why-it-exists, what-changed, problem-solution, gotcha, pattern, trade-off" },
                            "project": { "type": "string", "description": "Project name or path" },
                            "files": { "type": "array", "items": { "type": "string" }, "description": "Related file paths" },
                            "facts": { "type": "array", "items": { "type": "string" }, "description": "Extracted facts" }
                        }
                    }
                },
                {
                    "name": "search_memory",
                    "description": "Search persistent memories using FTS5 full-text search with BM25 ranking. Returns observations matching the query, filterable by project, type, and date range.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string", "description": "Search query (FTS5 syntax supported)" },
                            "project": { "type": "string", "description": "Filter by project" },
                            "type": { "type": "string", "description": "Filter by observation type" },
                            "limit": { "type": "integer", "description": "Max results (default 10, max 50)" },
                            "offset": { "type": "integer", "description": "Pagination offset" },
                            "date_start": { "type": "string", "description": "ISO 8601 date filter start" },
                            "date_end": { "type": "string", "description": "ISO 8601 date filter end" }
                        }
                    }
                },
                {
                    "name": "memory_timeline",
                    "description": "Retrieve observations chronologically around an anchor observation. Use search_memory first to find the anchor ID.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["anchor_id"],
                        "properties": {
                            "anchor_id": { "type": "integer", "description": "ID of the anchor observation" },
                            "depth_before": { "type": "integer", "description": "Observations before anchor (default 5)" },
                            "depth_after": { "type": "integer", "description": "Observations after anchor (default 5)" },
                            "project": { "type": "string", "description": "Filter by project" }
                        }
                    }
                }
            ]
        })
    }

    fn handle_tools_call(&self, request: &Value) -> Value {
        let params = request.get("params").unwrap_or(&Value::Null);
        let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let tool_input = params.get("arguments").unwrap_or(&Value::Null);

        // Extract file path from arguments (used by metrics tracking)
        let file_path = tool_input.get("path").and_then(|p| p.as_str());

        let start_time = std::time::Instant::now();

        let result = match tool_name {
            "workspace_state" => {
                let result = crate::tools::workspace_state::handle_workspace_state(
                    tool_input,
                    &self.state,
                );
                json!({
                    "content": [{
                        "type": "text",
                        "text": result.to_string()
                    }]
                })
            }
            "xray" => {
                match crate::tools::xray::handle_xray(
                    Arc::clone(&self.metrics),
                    &self.xray_server,
                ) {
                    Ok(result) => {
                        let envelope = Envelope::success(result, TrustLevel::Exact);
                        json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
                    }
                    Err(e) => {
                        let envelope = Envelope::<()>::error(ErrorCode::EInternalError, false, Some(e));
                        json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
                    }
                }
            }
            "save_memory" | "search_memory" | "memory_timeline" => {
                let db_arc = match &self.db {
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
                        // Rollback any partial transaction from the panicked thread
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
                    _ => unreachable!(),
                };
                json!({"content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}]})
            }
            _ => {
                json!({
                    "content": [{
                        "type": "text",
                        "text": "Tool not yet implemented"
                    }]
                })
            }
        };

        // Record metrics for xray dashboard (skip xray tool itself to avoid noise)
        if tool_name != "xray" {
            let result_text = result.to_string();
            let raw_bytes = result_text.len() as u64;
            let raw_tokens = raw_bytes / 4; // rough byte-to-token estimate
            // Estimate compression: skeleton/focused modes compress ~90%, others ~50%
            let compression = match tool_name {
                "smart_read" | "project_map" => 10, // 90% compression → 10% of raw
                "smart_run" => 8,                    // 92% compression
                "smart_edit" => 50,                  // edits are already compact
                _ => 80,                             // minimal compression
            };
            let compressed_tokens = raw_tokens * compression / 100;

            if let Ok(mut m) = self.metrics.lock() {
                m.record_tool_call(tool_name, file_path, raw_tokens, compressed_tokens);

                // Sync phase from session state
                if let Ok(state) = self.state.lock() {
                    let phase = match state.phase() {
                        crate::session::state::SessionPhase::Idle => "Idle",
                        crate::session::state::SessionPhase::Analyzing => "Analyzing",
                        crate::session::state::SessionPhase::Editing => "Editing",
                        crate::session::state::SessionPhase::Verifying => "Verifying",
                        crate::session::state::SessionPhase::Complete => "Complete",
                        crate::session::state::SessionPhase::Compacting => "Compacting",
                    };
                    m.record_timeline_event(
                        tool_name,
                        file_path,
                        raw_tokens,
                        compressed_tokens,
                        start_time.elapsed().as_millis() as u64,
                        phase,
                    );
                    m.set_phase(phase);
                    m.set_session_id(state.session_id());
                }
            }
        }

        result
    }

    fn respond(&self, id: Value, result: Value) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
        .to_string()
    }

    fn respond_error(&self, id: Value, code: i32, message: &str) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
        .to_string()
    }
}
