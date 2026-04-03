---
name: review
description: Reviewt Code-Änderungen in isoliertem Kontext.
argument-hint: [branch-oder-dateien]
context: fork
agent: code-reviewer
model: opus
---

Review: $ARGUMENTS

## Checkliste
- Security: Injection, XSS, Hardcoded Secrets, Auth-Bypass
- Performance: N+1, unnötige Allokationen, Lock Contention
- Correctness: Edge Cases, Error Handling, Race Conditions
- Maintainability: Naming, Complexity, Test Coverage

## Ausgabe
- CRITICAL | WARNING | SUGGESTION | GOOD
