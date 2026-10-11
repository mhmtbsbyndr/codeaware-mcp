use codeaware_mcp::server::McpServer;
use codeaware_mcp::v4::tokens::estimate_tokens;
use serde_json::{json, Value};

/// Baseline before schema compaction was 8241 bytes; budget is a 40% cut.
const TOOLS_LIST_BYTE_BUDGET: usize = 4946;

/// Measures the serialized tools/list payload that every session pays for.
#[test]
fn tools_list_payload_stays_within_budget() {
    let server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}});
    let raw = server.handle_message(&req.to_string()).expect("tools/list responds");
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    let tools = serde_json::to_string(&parsed["result"]["tools"]).unwrap();
    println!(
        "TOOLS_LIST bytes={} tools_array_bytes={} est_tokens={}",
        raw.len(),
        tools.len(),
        estimate_tokens(&tools)
    );
    assert!(
        raw.len() <= TOOLS_LIST_BYTE_BUDGET,
        "tools/list is {} bytes, budget {}",
        raw.len(),
        TOOLS_LIST_BYTE_BUDGET
    );
}
