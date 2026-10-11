use serde_json::{json, Value};
use codeaware_mcp::server::McpServer;

fn request(server: &McpServer, msg: Value) -> Option<Value> {
    server
        .handle_message(&msg.to_string())
        .map(|s| serde_json::from_str(&s).expect("response must be valid JSON"))
}

fn listed_tool_names(server: &McpServer) -> Vec<String> {
    let resp = request(server, json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}))
        .expect("tools/list must respond");
    resp["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

fn call_tool(server: &McpServer, name: &str, arguments: Value) -> Value {
    let resp = request(
        server,
        json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}),
    )
    .expect("tools/call must respond");
    assert!(resp.get("error").is_none(), "tools/call must not be a JSON-RPC error: {resp}");
    resp["result"].clone()
}

fn result_text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap_or("").to_string()
}

#[test]
fn test_initialize_response() {
    let server = McpServer::new();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let response = server.handle_message(&request.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed["result"]["serverInfo"]["name"], "codeaware");
    assert_eq!(parsed["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_tools_list_returns_all_tools() {
    let server = McpServer::new();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let response = server.handle_message(&request.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    let tools = parsed["result"]["tools"].as_array().unwrap();

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    assert!(tool_names.contains(&"project_map"));
    assert!(tool_names.contains(&"smart_read"));
    assert!(tool_names.contains(&"smart_edit"));
    assert!(tool_names.contains(&"smart_run"));
    assert!(tool_names.contains(&"workspace_state"));
    assert!(tool_names.contains(&"validate_config"));
    assert!(tool_names.contains(&"xray"));
    assert!(tool_names.contains(&"save_memory"));
    assert!(tool_names.contains(&"search_memory"));
    assert!(tool_names.contains(&"memory_timeline"));
    assert!(tool_names.contains(&"git_diff"));
    assert!(tool_names.contains(&"git_blame"));
    assert!(tool_names.contains(&"git_changelog"));
    assert!(tool_names.contains(&"smart_refactor"));
    assert!(tool_names.contains(&"summarize_memory"));
    assert!(tool_names.contains(&"test_coverage_map"));
    // session_status was removed: it has no handler (only a result struct exists).
    assert!(!tool_names.contains(&"session_status"));
    assert!(!tool_names.contains(&"codeaware__workspace_context"));
    assert_eq!(tools.len(), 16);
}

#[test]
fn test_every_listed_tool_dispatches() {
    let server = McpServer::new();
    let names = listed_tool_names(&server);
    assert!(!names.is_empty());
    for name in names {
        // Called with no arguments: handlers may reject the input, but none may
        // fall through to the unknown-tool branch.
        let result = call_tool(&server, &name, json!({}));
        let text = result_text(&result);
        assert!(!text.contains("Unknown tool"), "{name} is not dispatched: {text}");
        assert!(!text.contains("not yet implemented"), "{name} is not implemented: {text}");
    }
}

#[test]
fn test_unknown_tool_returns_is_error() {
    let server = McpServer::new();
    let result = call_tool(&server, "no_such_tool", json!({}));
    assert_eq!(result["isError"], true);
    assert!(result_text(&result).contains("no_such_tool"));
}

#[test]
fn test_smart_read_dispatches_real_file() {
    let server = McpServer::new();
    let result = call_tool(&server, "smart_read", json!({"path": "Cargo.toml"}));
    assert!(result.get("isError").is_none() || result["isError"] == false, "{result}");
    assert!(result_text(&result).contains("codeaware"));
}

#[test]
fn test_ping_returns_empty_result() {
    let server = McpServer::new();
    let resp = request(&server, json!({"jsonrpc": "2.0", "id": 5, "method": "ping"})).unwrap();
    assert_eq!(resp["id"], 5);
    assert_eq!(resp["result"], json!({}));
    assert!(resp.get("error").is_none());
}

#[test]
fn test_messages_without_id_get_no_response() {
    let server = McpServer::new();
    assert!(request(&server, json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
    assert!(request(&server, json!({"jsonrpc": "2.0", "method": "ping"})).is_none());
    assert!(request(&server, json!({"jsonrpc": "2.0", "method": "tools/list"})).is_none());
}

#[test]
fn test_client_response_gets_no_response() {
    let server = McpServer::new();
    assert!(request(&server, json!({"jsonrpc": "2.0", "id": 42, "result": {}})).is_none());
    assert!(request(&server, json!({"jsonrpc": "2.0", "id": 43, "error": {"code": -1, "message": "x"}})).is_none());
}

#[test]
fn test_batch_elements_are_each_handled() {
    let server = McpServer::new();
    let batch = json!([
        {"jsonrpc": "2.0", "id": 1, "method": "ping"},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "nonexistent/method"}
    ]);
    let resp = server.handle_message(&batch.to_string()).expect("batch with requests must respond");
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    let arr = parsed.as_array().expect("batch response is an array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], 1);
    assert_eq!(arr[0]["result"], json!({}));
    assert_eq!(arr[1]["id"], 2);
    assert_eq!(arr[1]["error"]["code"], -32601);
}

#[test]
fn test_batch_of_only_notifications_gets_no_response() {
    let server = McpServer::new();
    let batch = json!([{"jsonrpc": "2.0", "method": "notifications/initialized"}]);
    assert!(server.handle_message(&batch.to_string()).is_none());
}

#[test]
fn test_empty_batch_is_invalid_request() {
    let server = McpServer::new();
    let resp: Value = serde_json::from_str(&server.handle_message("[]").unwrap()).unwrap();
    assert_eq!(resp["id"], Value::Null);
    assert_eq!(resp["error"]["code"], -32600);
}

#[test]
fn test_unknown_method_returns_error() {
    let server = McpServer::new();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "nonexistent/method",
        "params": {}
    });
    let response = server.handle_message(&request.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert!(parsed.get("error").is_some());
    assert_eq!(parsed["error"]["code"], -32601);
}

#[test]
fn test_smart_run_dispatches_command() {
    let server = McpServer::new();
    let result = call_tool(&server, "smart_run", json!({"command": "echo codeaware_probe"}));
    assert!(result.get("isError").is_none() || result["isError"] == false, "{result}");
    assert!(result_text(&result).contains("codeaware_probe"));
}
