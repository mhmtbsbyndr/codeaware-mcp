use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use crate::hooks::profiles::{self, Profile};
use crate::session::state::SessionState;
use crate::session::persistence::SessionDb;
use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;

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
        let db = Self::open_db();

        // Auto-start XRay dashboard on server init
        let xray = XrayServer::start(Arc::clone(&metrics), db.clone()).ok();
        if let Some(ref srv) = xray {
            eprintln!("XRay dashboard: {}", srv.url());
        }

        // Context injection: load memories from previous sessions
        let state = Arc::new(Mutex::new(SessionState::new(".")));
        if let Some(ref db_arc) = db {
            if let Ok(db_guard) = db_arc.lock() {
                if let Some(ctx) = crate::hooks::context_injection::inject_context(&db_guard, ".") {
                    if let Ok(mut s) = state.lock() {
                        s.set_injected_context(ctx);
                    }
                }
            }
        }

        // Session persistence: load recent session state if available
        if let Some(prev_state) = crate::hooks::session_persistence::load_recent_session(".") {
            eprintln!("CodeAware: Restored previous session state for project");
            if let Ok(mut s) = state.lock() {
                s.set_injected_context(
                    format!(
                        "Resumed session state: {}",
                        serde_json::to_string(&prev_state).unwrap_or_default()
                    ),
                );
            }
        }

        McpServer {
            state,
            metrics,
            xray_server: Mutex::new(xray),
            db,
        }
    }

    pub fn new_with_state(state: Arc<Mutex<SessionState>>) -> Self {
        let metrics = Arc::new(Mutex::new(MetricsState::new()));
        let db = Self::open_db();

        let xray = XrayServer::start(Arc::clone(&metrics), db.clone()).ok();
        if let Some(ref srv) = xray {
            eprintln!("XRay dashboard: {}", srv.url());
        }

        // Context injection: load memories from previous sessions
        let project_path = state.lock().map(|s| s.project_path().to_string()).unwrap_or_else(|_| ".".to_string());
        if let Some(ref db_arc) = db {
            if let Ok(db_guard) = db_arc.lock() {
                if let Some(ctx) = crate::hooks::context_injection::inject_context(&db_guard, &project_path) {
                    if let Ok(mut s) = state.lock() {
                        s.set_injected_context(ctx);
                    }
                }
            }
        }

        // Session persistence: load recent session state if available
        if let Some(prev_state) = crate::hooks::session_persistence::load_recent_session(&project_path) {
            eprintln!("CodeAware: Restored previous session state for project");
            if let Ok(mut s) = state.lock() {
                s.set_injected_context(
                    format!(
                        "Resumed session state: {}",
                        serde_json::to_string(&prev_state).unwrap_or_default()
                    ),
                );
            }
        }

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

    /// Handle one incoming line (a JSON-RPC message or batch) and return the reply line.
    ///
    /// Rules:
    /// - Unparseable input: -32700 with id null.
    /// - Batch arrays: each element is handled independently. The replies are returned
    ///   as one JSON array in request order; elements that need no reply are omitted.
    ///   If no element needs a reply, nothing is returned. An empty batch is -32600 (id null).
    /// - Messages without an `id` (notifications, including `notifications/*`) get no reply.
    /// - Client responses (`id` + `result`/`error`, no `method`) get no reply.
    /// - `ping` returns an empty result object `{}`.
    pub fn handle_message(&self, message: &str) -> Option<String> {
        let parsed: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(_) => return Some(self.respond_error(Value::Null, -32700, "Parse error").to_string()),
        };

        if let Value::Array(items) = &parsed {
            if items.is_empty() {
                return Some(self.respond_error(Value::Null, -32600, "Invalid Request: empty batch").to_string());
            }
            let replies: Vec<Value> = items.iter().filter_map(|item| self.handle_single(item)).collect();
            if replies.is_empty() {
                return None;
            }
            return Some(Value::Array(replies).to_string());
        }

        self.handle_single(&parsed).map(|reply| reply.to_string())
    }

    /// Handle a single JSON-RPC message object. Returns `None` when no reply is due.
    fn handle_single(&self, msg: &Value) -> Option<Value> {
        if !msg.is_object() {
            return Some(self.respond_error(Value::Null, -32600, "Invalid Request: not an object"));
        }

        let method = match msg.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                // Client response to a server-initiated request: never answered.
                // Any other message without an id is a notification: never answered.
                let id = msg.get("id")?;
                if msg.get("result").is_some() || msg.get("error").is_some() {
                    return None;
                }
                return Some(self.respond_error(id.clone(), -32600, "Invalid Request: missing method"));
            }
        };

        // Requests without an id are notifications and expect no response.
        let id = msg.get("id").cloned()?;

        match method {
            "initialize" => Some(self.respond(id, self.handle_initialize())),
            "ping" => Some(self.respond(id, json!({}))),
            "tools/list" => Some(self.respond(id, self.handle_tools_list())),
            "tools/call" => Some(self.respond(id, self.handle_tools_call(msg))),
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
                    "description": "Map the project directory structure.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "depth": { "type": "integer" },
                            "include_symbols": { "type": "boolean" },
                            "filter_language": { "type": "string" },
                            "task_context": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "smart_read",
                    "description": "Read a file with focused context.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["path"],
                        "properties": {
                            "path": { "type": "string" },
                            "mode": { "type": "string", "description": "auto, skeleton, focused, full or diff" },
                            "focus": { "type": "string" },
                            "lines": { "type": "array", "items": { "type": "integer" } },
                            "scope": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "smart_edit",
                    "description": "Edit a file using intelligent strategies.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["path"],
                        "properties": {
                            "path": { "type": "string" },
                            "strategy": { "type": "string", "description": "text or lines ('symbol' unsupported)" },
                            "symbol": { "type": "string" },
                            "line_range": { "type": "array", "items": { "type": "integer" } },
                            "edits": { "type": "array", "items": { "type": "object" } },
                            "new_content": { "type": "string" },
                            "dry_run": { "type": "boolean" },
                            "expected_hash": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "smart_run",
                    "description": "Run a command with compressed output capture.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["command"],
                        "properties": {
                            "command": { "type": "string" },
                            "max_output_lines": { "type": "integer" },
                            "capture_relevant_code": { "type": "boolean" },
                            "scan_secrets": { "type": "boolean" }
                        }
                    }
                },
                {
                    "name": "workspace_state",
                    "description": "Read, write or clear typed workspace state slots.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["action", "slot"],
                        "properties": {
                            "action": { "type": "string", "enum": ["read", "write", "clear"] },
                            "slot": {
                                "type": "string",
                                "enum": [
                                    "recent_targets",
                                    "error_signatures",
                                    "co_access_candidates",
                                    "verification_state",
                                    "active_task"
                                ]
                            },
                            "value": { "type": "object", "description": "Only for action=write" }
                        }
                    }
                },
                {
                    "name": "codeaware_v4",
                    "description": "Semantic code context. op: task_context, find_symbol, find_callers, find_tests, diff_impact.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["op"],
                        "properties": {
                            "op": { "type": "string", "enum": ["task_context", "find_symbol", "find_callers", "find_tests", "diff_impact"] },
                            "goal": { "type": "string" },
                            "task_id": { "type": "string" },
                            "query": { "type": "string" },
                            "symbol": { "type": "string" },
                            "changed_path": { "type": "string" },
                            "repo_root": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "skill_lookup",
                    "description": "Best matching skill excerpt for a goal.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["goal"],
                        "properties": {
                            "goal": { "type": "string" },
                            "max_results": { "type": "integer" },
                            "max_bytes": { "type": "integer" }
                        }
                    }
                },
                {
                    "name": "validate_config",
                    "description": "Validate CodeAware configuration files.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "xray",
                    "description": "Open a live browser dashboard of token and session metrics.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "save_memory",
                    "description": "Save an observation to persistent, FTS5-searchable memory.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["text"],
                        "properties": {
                            "text": { "type": "string" },
                            "title": { "type": "string" },
                            "type": { "type": "string", "enum": ["bugfix","feature","refactor","change","discovery","decision"], "description": "Default discovery" },
                            "concepts": { "type": "array", "items": { "type": "string" } },
                            "project": { "type": "string" },
                            "files": { "type": "array", "items": { "type": "string" } },
                            "facts": { "type": "array", "items": { "type": "string" } }
                        }
                    }
                },
                {
                    "name": "search_memory",
                    "description": "Full-text search memories (FTS5, BM25-ranked).",
                    "inputSchema": {
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": { "type": "string" },
                            "project": { "type": "string" },
                            "type": { "type": "string" },
                            "limit": { "type": "integer", "description": "Default 10, max 50" },
                            "offset": { "type": "integer" },
                            "date_start": { "type": "string" },
                            "date_end": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "memory_timeline",
                    "description": "Observations chronologically around an anchor. Use search_memory first.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["anchor_id"],
                        "properties": {
                            "anchor_id": { "type": "integer" },
                            "depth_before": { "type": "integer" },
                            "depth_after": { "type": "integer" },
                            "project": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "git_diff",
                    "description": "Git diff between two refs with add/delete counts.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "base": { "type": "string", "description": "Default HEAD~1" },
                            "head": { "type": "string", "description": "Default HEAD" }
                        }
                    }
                },
                {
                    "name": "git_blame",
                    "description": "Git blame for a file or line range.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["file"],
                        "properties": {
                            "file": { "type": "string" },
                            "start_line": { "type": "integer" },
                            "end_line": { "type": "integer" }
                        }
                    }
                },
                {
                    "name": "git_changelog",
                    "description": "Changelog grouped by conventional commit type.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "base": { "type": "string", "description": "Base ref, e.g. v1.0.0" },
                            "head": { "type": "string", "description": "Default HEAD" },
                            "limit": { "type": "integer", "description": "Default 50" }
                        }
                    }
                },
                {
                    "name": "summarize_memory",
                    "description": "Cluster and dedupe memories into summaries.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project": { "type": "string" },
                            "force": { "type": "boolean" }
                        }
                    }
                },
                {
                    "name": "smart_refactor",
                    "description": "AST-aware project-wide rename. Dry-run by default.",
                    "inputSchema": {
                        "type": "object",
                        "required": ["old_name", "new_name"],
                        "properties": {
                            "operation": { "type": "string", "default": "rename" },
                            "old_name": { "type": "string" },
                            "new_name": { "type": "string" },
                            "path": { "type": "string" },
                            "dry_run": { "type": "boolean", "description": "Default true" }
                        }
                    }
                },
                {
                    "name": "test_coverage_map",
                    "description": "Heuristic map of functions with and without tests.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "language": { "type": "string" }
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
        let file_path = tool_input.get("path").and_then(|p| p.as_str());

        let start_time = std::time::Instant::now();

        let result = crate::tools::dispatch::dispatch_tool(
            tool_name,
            tool_input,
            &self.state,
            &self.db,
            &self.xray_server,
            &self.metrics,
        );

        if tool_name != "xray" {
            self.record_post_tool_metrics(tool_name, file_path, &result, start_time);
        }

        result
    }

    /// Record metrics, auto-observe, and governance hooks after a tool call.
    fn record_post_tool_metrics(
        &self,
        tool_name: &str,
        file_path: Option<&str>,
        result: &Value,
        start_time: std::time::Instant,
    ) {
        let result_text = result.to_string();
        let raw_bytes = result_text.len() as u64;
        let raw_tokens = raw_bytes / 4;
        let compression = match tool_name {
            "smart_read" | "project_map" => 10,
            "smart_run" => 8,
            "smart_edit" => 50,
            _ => 80,
        };
        let compressed_tokens = raw_tokens * compression / 100;

        let profile = profiles::get_profile();

        if let Ok(mut m) = self.metrics.lock() {
            m.record_tool_call(tool_name, file_path, raw_tokens, compressed_tokens);

            if profile != Profile::Minimal {
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

            m.check_compaction_hints();
        }

        if profile == Profile::Rich {
            eprintln!(
                "CodeAware [rich]: {} on {} — {} raw tokens, {} compressed",
                tool_name,
                file_path.unwrap_or("-"),
                raw_tokens,
                compressed_tokens
            );
        }

        if let Some(ref db_arc) = self.db {
            if let Ok(db_guard) = db_arc.lock() {
                let session_id = self.state.lock()
                    .map(|s| s.session_id().to_string())
                    .unwrap_or_default();
                crate::hooks::auto_observe::record_auto_observation(
                    &db_guard,
                    tool_name,
                    file_path,
                    &session_id,
                    &result_text,
                );
            }
        }

        if crate::hooks::governance::is_governance_enabled() {
            if let Some(event) = crate::hooks::governance::check_governance(
                tool_name,
                file_path,
                &result_text,
            ) {
                crate::hooks::governance::log_governance_event(&event);
            }
        }
    }

    fn respond(&self, id: Value, result: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }

    fn respond_error(&self, id: Value, code: i32, message: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
    }
}
