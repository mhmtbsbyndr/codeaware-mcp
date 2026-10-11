use codeaware_mcp::server::McpServer;
use codeaware_mcp::v4::tokens::estimate_tokens;
use serde_json::json;

/// Measures the serialized tools/list payload that every session pays for.
#[test]
fn tools_list_size_report() {
    let server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}});
    let resp = server.handle_message(&req.to_string()).expect("tools/list responds");
    let result = resp.len();
    let tools: serde_json::Value = serde_json::from_str(&resp).unwrap();
    let list = serde_json::to_string(&tools["result"]["tools"]).unwrap();
    println!(
        "TOOLS_LIST bytes={} tools_array_bytes={} est_tokens={}",
        result,
        list.len(),
        estimate_tokens(&list)
    );
}
