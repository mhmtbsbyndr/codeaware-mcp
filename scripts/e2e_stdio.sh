#!/usr/bin/env bash
# End-to-end check of the release binary over stdio (JSON-RPC, one message per line).
# Exits non-zero if any response is not valid JSON-RPC, a tool call fails, or a tool
# answers with "not implemented".
#
# Usage: scripts/e2e_stdio.sh [path/to/codeaware-mcp]
# Requires: jq. The server resolves relative paths against its working directory,
# so the script runs it from the repository root.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${1:-$REPO/target/release/codeaware-mcp}"
READ_FILE="Cargo.toml"

command -v jq >/dev/null 2>&1 || { echo "FAIL: jq is required" >&2; exit 1; }
[[ -x "$BIN" ]] || { echo "FAIL: binary not found: $BIN (run cargo build --release --bin codeaware-mcp)" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
REQ="$WORK/requests.jsonl"
RESP="$WORK/responses.jsonl"

cat > "$REQ" <<JSON
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e_stdio","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"smart_read","arguments":{"path":"$READ_FILE","mode":"full"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"smart_read","arguments":{"path":"$READ_FILE","mode":"full"}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"project_map","arguments":{"path":"src/tools","depth":1}}}
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"validate_config","arguments":{}}}
JSON

# Run from the repo root: the server's project path is its working directory.
(cd "$REPO" && "$BIN" < "$REQ" > "$RESP" 2> "$WORK/stderr.txt")
server_status=$?

failures=0
pass() { echo "PASS  $1"; }
fail() { echo "FAIL  $1"; failures=$((failures + 1)); }

echo "server exit status: $server_status (stdin closed, so 0 is expected)"
[[ $server_status -eq 0 ]] || fail "server exited with status $server_status"

# 1. Every response line is valid JSON-RPC 2.0 with the expected id and a result or error.
expected_ids=(1 2 3 4 5 6)
mapfile -t lines < "$RESP"
if [[ ${#lines[@]} -ne ${#expected_ids[@]} ]]; then
    fail "expected ${#expected_ids[@]} responses, got ${#lines[@]}"
fi
for i in "${!lines[@]}"; do
    id="${expected_ids[$i]:-?}"
    if echo "${lines[$i]}" | jq -e --argjson id "$id" \
        '.jsonrpc == "2.0" and .id == $id and ((has("result")) or (has("error")))' >/dev/null 2>&1; then
        pass "response id=$id is valid JSON-RPC"
    else
        fail "response id=$id is not valid JSON-RPC: ${lines[$i]:0:200}"
    fi
done

# Response lines are in request order; these are their 0-based indices.
IDX_INIT=0; IDX_LIST=1; IDX_READ1=2; IDX_READ2=3; IDX_PMAP=4; IDX_VALID=5
get() { echo "${lines[$1]:-}"; }
text_of() { get "$1" | jq -r '.result.content[0].text // empty'; }
check_tool_ok() { # index, label
    if get "$1" | jq -e '.result.isError != true' >/dev/null 2>&1; then
        pass "$2 returned no error"
    else
        fail "$2 returned isError: $(text_of "$1" | head -c 200)"
    fi
    if text_of "$1" | grep -qi "not implemented"; then
        fail "$2 contains 'not implemented'"
    else
        pass "$2 has no 'not implemented' text"
    fi
}

# 2. tools/list: count and size of the result object.
tools_count=$(get $IDX_LIST | jq '.result.tools | length' 2>/dev/null || echo 0)
tools_bytes=$(get $IDX_LIST | jq -c '.result' 2>/dev/null | wc -c)
if [[ "$tools_count" -gt 0 ]]; then
    pass "tools/list advertises $tools_count tools (.result = $tools_bytes bytes)"
else
    fail "tools/list returned no tools"
fi

# 3. smart_read: first call serves the full body, second call the unchanged marker.
check_tool_ok $IDX_READ1 "smart_read #1"
if [[ "$(text_of $IDX_READ1 | jq -r '.mode_used' 2>/dev/null)" == "full" ]] \
    && cmp -s <(text_of $IDX_READ1 | jq -r '.content') "$REPO/$READ_FILE"; then
    pass "smart_read #1 is the full body of $READ_FILE"
else
    fail "smart_read #1 is not the full body of $READ_FILE"
fi

check_tool_ok $IDX_READ2 "smart_read #2"
marker=$(text_of $IDX_READ2 | jq -r '.content // empty' 2>/dev/null)
if [[ "$(text_of $IDX_READ2 | jq -r '.mode_used' 2>/dev/null)" == "unchanged" ]] \
    && [[ "$marker" =~ ^unchanged\ since\ [0-9a-f]{8}$ ]]; then
    pass "smart_read #2 returns marker: $marker"
else
    fail "smart_read #2 did not return the unchanged marker (got: ${marker:0:120})"
fi

# 4. project_map and validate_config.
check_tool_ok $IDX_PMAP "project_map"
check_tool_ok $IDX_VALID "validate_config"
echo "INFO  validate_config text: $(text_of $IDX_VALID | head -c 160)"

echo
if [[ $failures -eq 0 ]]; then
    echo "E2E RESULT: PASS"
    exit 0
fi
echo "E2E RESULT: FAIL ($failures failure(s))"
exit 1
