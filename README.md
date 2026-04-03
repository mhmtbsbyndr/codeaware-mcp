# codeaware-mcp

A context-efficient MCP server for Claude Code. Reduces token consumption through smart compression, incremental reads, and session-aware orchestration.

## What it does

Instead of Claude reading raw files and terminal output repeatedly, codeaware-mcp sits between Claude and the filesystem and:

- **Compresses tool output** — test results, build errors, linter output, git diffs reduced to dense summaries
- **Reads files intelligently** — skeleton mode for large files, focused extraction for specific functions, FTS5 search for compaction recovery
- **Tracks session state** — file access patterns, error signatures, workspace slots for cross-tool context
- **Validates config** — structured findings with severity, score, and auto-fix flags
- **Enforces security** — path traversal protection, 14 secret scanner patterns, deny-list enforcement

## Tools

| Tool | Purpose |
|------|---------|
| `smart_read` | Context-aware file reading (full / skeleton / focused) |
| `smart_edit` | Edit with impact analysis and conflict detection |
| `smart_run` | Run commands with compressed output |
| `project_map` | Compressed project structure overview |
| `workspace_state` | 5 typed slots for cross-tool session state |
| `session_status` | Session summary and compaction recovery |
| `validate_config` | Config validation with structured findings |

## Features

- **7 MCP tools** with JSON-RPC envelope and typed error codes
- **6 tree-sitter languages**: Rust, Python, TypeScript, JavaScript, Go, PHP, Swift
- **SQLite persistence** with WAL mode and FTS5 for session event indexing
- **14 secret scanner patterns**: AWS, GitHub, OpenAI, Anthropic, JWT, Stripe, Twilio, PEM, and more
- **Skills & Agents**: `/analyze`, `/fix`, `/review`, `/project-map` + preloaded `smart-read`, `smart-edit`, `smart-run`, `gotchas`
- **Hook events**: PostToolUse, PostToolUseFailure, PreCompact, PostCompact, SubagentStop, Stop
- **175 tests**, 0 failures

## Install

```bash
git clone https://github.com/mhmtbsbyndr/codeaware-mcp
cd codeaware-mcp
cargo build --release
cp target/release/codeaware-mcp /usr/local/bin/
```

## Configure

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "codeaware": {
      "command": "codeaware-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

Optionally create `.codeaware.toml` in your project root for per-project settings (enforcement, compression thresholds, language config).

## CLAUDE.md integration

```markdown
<important if="reading or editing files">
Prefer smart_read for files > 50 LOC, smart_run for tests/builds, smart_edit when impact analysis is needed.
</important>
```

## License

MIT
