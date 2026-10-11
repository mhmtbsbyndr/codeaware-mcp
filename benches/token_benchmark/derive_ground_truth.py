#!/usr/bin/env python3
"""Derive the token-benchmark task suite from git history.

Ground truth per task:
- The commit's MODIFIED .rs files that also exist in the parent are the relevant set.
- ADDED .rs files are recorded as excluded: they cannot be discovered before the
  change, so no retrieval variant could find them.
- The index input is always the parent tree (see tests/tokens/runner). The commit
  itself is never an index input, so there is no leakage.

Goals are written from the commit subject or body (the request a user would make).

Usage (from the repo root):  python3 benches/token_benchmark/derive_ground_truth.py
"""
import json
import pathlib
import subprocess

REPO = pathlib.Path(__file__).resolve().parents[2]
OUT = REPO / "benches/token_benchmark/tasks.json"

# (short commit, goal). Commits without a modified .rs file are not eligible.
CURATED = [
    ("fc1e59c", "Make get_task_context semantic-first so goal-relevant symbols come before file summaries"),
    ("d9c5c6a", "Complete phase 1 of get_task_context with candidate discovery and task trace persistence"),
    ("e838f13", "Harden v4 tool error handling"),
    ("b6e6f31", "Wire the v4 semantic tools into the MCP tool dispatcher"),
    ("d31b945", "Export the final production-hardening runtime modules"),
    ("977950c", "Prepare v4.0.0 release metadata"),
    ("57e5977", "Export architecture memory and semantic runtime layers"),
    ("b861740", "Export semantic MCP-style APIs"),
    ("3f1b8e2", "Export the semantic context assembler"),
    ("09e5d0a", "Export the semantic index runtime"),
    ("043e5c7", "Export call graph and semantic retrieval"),
    ("06c2cf8", "Export phase 2 semantic graph modules"),
    ("f9e001d", "Export v4 symbol models"),
    ("b4f37c8", "Export the v4 discovery module"),
    ("2ef656a", "Continue token-saving work: trim tool schemas and compressor output, port hooks and memory"),
    ("627e547", "Fix smart_edit and smart_refactor issues, harden xray and validate session ids"),
    ("3679e54", "Fix v4 discovery and call graph, enforce deny-list and path checks, fix git option injection and blame"),
    ("52c155a", "Apply review fixes to deny-list path enforcement, path resolution and v4 semantic context and storage"),
    ("402f070", "Review fixes across git output compression, auto-observe hook, tool dispatch, git intelligence and v4 call graph"),
    ("8c7e7d9", "Fix test compile errors, UTF-8 truncation panic, git option injection, dispatch wiring and deny-list enforcement"),
]


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(REPO), *args], check=True, capture_output=True, text=True
    ).stdout


def main() -> None:
    tasks = []
    for index, (short, goal) in enumerate(CURATED, start=1):
        commit = git("rev-parse", short).strip()
        parent = git("rev-parse", commit + "^").strip()
        subject = git("log", "-1", "--format=%s", commit).strip()

        modified, added = [], []
        for line in git("diff-tree", "--no-commit-id", "--name-status", "-r", commit).splitlines():
            status, path = line.split("\t", 1)
            if not path.endswith(".rs"):
                continue
            if status == "M":
                # Must exist in the parent tree, otherwise it is not discoverable.
                git("cat-file", "-e", f"{parent}:{path}")
                modified.append(path)
            elif status == "A":
                added.append(path)

        if not modified:
            raise SystemExit(f"{short}: no modified .rs file, not eligible")

        tasks.append({
            "id": f"T{index:02d}",
            "commit": commit,
            "parent": parent,
            "subject": subject,
            "goal": goal,
            "relevant_files": sorted(modified),
            "excluded_added_files": sorted(added),
        })

    document = {
        "schema_version": 1,
        "source": "git history: relevant = modified .rs files; index input = parent tree; commit excluded",
        "tasks": tasks,
    }
    OUT.write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {len(tasks)} tasks to {OUT}")


if __name__ == "__main__":
    main()
