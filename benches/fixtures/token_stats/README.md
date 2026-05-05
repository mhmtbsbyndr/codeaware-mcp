# Token Stats Benchmark Fixtures

This directory contains deterministic fixtures for compression and token-savings benchmarks.

## Purpose

The goal is to make token-saving claims measurable and reproducible.

Each fixture should include:

- a raw input file;
- an expected compressed representation;
- metadata describing category, tool, language, and expected savings range.

## Naming Convention

```text
<category>_<tool>_<short_name>.raw.txt
<category>_<tool>_<short_name>.compressed.txt
<category>_<tool>_<short_name>.meta.json
```

Example:

```text
file_read_smart_read_rust_module.raw.txt
file_read_smart_read_rust_module.compressed.txt
file_read_smart_read_rust_module.meta.json
```

## Metadata Shape

```json
{
  "category": "file_read",
  "tool": "smart_read",
  "language": "rust",
  "subject": "src/server.rs",
  "expected_min_savings_ratio": 0.5,
  "expected_max_savings_ratio": 0.98
}
```

## Benchmark Rules

- Fixtures must be small enough to review in PRs.
- Fixtures must be deterministic.
- Expected savings should be ranges, not exact values.
- Do not include private or customer code.
- Prefer synthetic examples modeled after common project shapes.

## Planned Fixture Categories

- `file_read`
- `command_output`
- `git_diff`
- `search_output`
- `memory_resume`
- `tool_schema`

## Future Tool

These fixtures are intended for the planned MCP tool:

```text
benchmark_compression
```
