use codeaware_mcp::server::McpServer;
use serde_json::Value;

#[test]
fn test_xray_tool_in_tools_list() {
    let server = McpServer::new();
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let response = server.handle_message(msg).unwrap();
    let parsed: Value = serde_json::from_str(&response).unwrap();
    let tools = parsed["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"xray"), "xray tool should be in tools list, got: {:?}", names);
}

#[test]
fn test_xray_tool_returns_url() {
    let server = McpServer::new();
    let msg = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"xray","arguments":{}}}"#;
    let response = server.handle_message(msg).unwrap();
    let parsed: Value = serde_json::from_str(&response).unwrap();
    let text = parsed["result"]["content"][0]["text"].as_str().unwrap();
    let envelope: Value = serde_json::from_str(text).unwrap();
    assert_eq!(envelope["ok"], true);
    let url = envelope["data"]["url"].as_str().unwrap();
    assert!(url.starts_with("http://127.0.0.1:"), "URL should be localhost, got: {}", url);
}
