use codeaware_mcp::server::McpServer;

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
    assert!(tool_names.contains(&"session_status"));
    assert!(tool_names.contains(&"workspace_state"));
    assert!(tool_names.contains(&"validate_config"));
    assert!(tool_names.contains(&"xray"));
    assert_eq!(tools.len(), 8);
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
