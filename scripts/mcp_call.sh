#!/usr/bin/env bash
# Call one CodeAware MCP tool over stdio and print the tool result text.
# Usage: scripts/mcp_call.sh <tool_name> '<json-arguments>'
# Needs target/release/codeaware-mcp (cargo build --release) and jq.
set -euo pipefail
BIN="$(dirname "$0")/../target/release/codeaware-mcp"
TOOL="$1"
ARGS="${2:-{\}}"
{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}'
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  jq -cn --arg t "$TOOL" --argjson a "$ARGS" '{jsonrpc:"2.0",id:2,method:"tools/call",params:{name:$t,arguments:$a}}'
} | "$BIN" 2>/dev/null | jq -r 'select(.id==2) | .result.content[0].text // (.error|tostring)'
