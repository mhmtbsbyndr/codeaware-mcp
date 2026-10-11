# Token benchmark results

- Estimator: token_stats::estimate_tokens: max(ceil(chars/4), ceil(words*4/3)); heuristic, not a tokenizer
- Repo HEAD: `d7ed321`
- Tasks: 20, top-k: 5, budget: 4000 tokens

ESTIMATE ONLY: these are heuristic token estimates, not real tokenizer counts.

## Summary per variant

| Variant | Role | Mean tokens | Total tokens | Mean files returned | Mean Recall@k | Covered within budget |
|---|---|---:|---:|---:|---:|---:|
| baseline_full_relevant | reference | 6767 | 135333 | 4.5 | 1.00 | 15/20 |
| naive_full_topk | achievable | 13243 | 264869 | 5.0 | 0.21 | 0/20 |
| v4_get_task_context | achievable | 2960 | 59202 | 11.7 | 0.17 | 2/20 |
| v4_semantic_only | achievable | 2477 | 49536 | 11.2 | 0.17 | 2/20 |
| smart_read_skeleton_topk | achievable | 3157 | 63131 | 5.0 | 0.21 | 2/20 |
| smart_read_skeleton_relevant | reference | 2120 | 42395 | 4.5 | 1.00 | 16/20 |

## Per task (tokens / Recall@k)

| Task | Rel. | baseline_full_relevant tok | baseline_full_relevant R | naive_full_topk tok | naive_full_topk R | v4_get_task_context tok | v4_get_task_context R | v4_semantic_only tok | v4_semantic_only R | smart_read_skeleton_topk tok | smart_read_skeleton_topk R | smart_read_skeleton_relevant tok | smart_read_skeleton_relevant R |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| T01 | 1 | 1744 | 1.00 | 34637 | 1.00 | 3213 | 0.00 | 2780 | 0.00 | 5677 | 1.00 | 387 | 1.00 |
| T02 | 1 | 1191 | 1.00 | 15297 | 1.00 | 3311 | 1.00 | 2879 | 1.00 | 2842 | 1.00 | 387 | 1.00 |
| T03 | 3 | 3474 | 1.00 | 15197 | 0.33 | 3357 | 0.33 | 2940 | 0.33 | 3533 | 0.33 | 1092 | 1.00 |
| T04 | 1 | 1269 | 1.00 | 13371 | 1.00 | 3255 | 0.00 | 2831 | 0.00 | 2415 | 1.00 | 168 | 1.00 |
| T05 | 1 | 526 | 1.00 | 18158 | 0.00 | 1582 | 0.00 | 2 | 0.00 | 4358 | 0.00 | 647 | 1.00 |
| T06 | 1 | 593 | 1.00 | 9011 | 0.00 | 1171 | 0.00 | 753 | 0.00 | 2195 | 0.00 | 722 | 1.00 |
| T07 | 1 | 455 | 1.00 | 4956 | 0.00 | 3317 | 0.00 | 2894 | 0.00 | 1855 | 0.00 | 569 | 1.00 |
| T08 | 1 | 412 | 1.00 | 4758 | 0.00 | 3290 | 0.00 | 2872 | 0.00 | 1508 | 0.00 | 554 | 1.00 |
| T09 | 1 | 386 | 1.00 | 6562 | 0.00 | 3280 | 0.00 | 2861 | 0.00 | 1983 | 0.00 | 523 | 1.00 |
| T10 | 1 | 353 | 1.00 | 4651 | 0.00 | 3115 | 0.00 | 2697 | 0.00 | 1997 | 0.00 | 486 | 1.00 |
| T11 | 1 | 307 | 1.00 | 4555 | 0.00 | 3143 | 1.00 | 2723 | 1.00 | 2103 | 0.00 | 435 | 1.00 |
| T12 | 1 | 246 | 1.00 | 3874 | 0.00 | 2768 | 0.00 | 2348 | 0.00 | 1429 | 0.00 | 366 | 1.00 |
| T13 | 1 | 227 | 1.00 | 19874 | 0.00 | 3099 | 0.00 | 2683 | 0.00 | 4667 | 0.00 | 346 | 1.00 |
| T14 | 1 | 208 | 1.00 | 19542 | 0.00 | 1371 | 0.00 | 953 | 0.00 | 3763 | 0.00 | 324 | 1.00 |
| T15 | 13 | 26622 | 1.00 | 11891 | 0.00 | 3334 | 0.00 | 2901 | 0.00 | 4085 | 0.00 | 7043 | 1.00 |
| T16 | 1 | 2274 | 1.00 | 20541 | 0.00 | 3389 | 0.00 | 2959 | 0.00 | 3861 | 0.00 | 821 | 1.00 |
| T17 | 15 | 23728 | 1.00 | 15271 | 0.13 | 3269 | 0.13 | 2833 | 0.13 | 4287 | 0.13 | 7018 | 1.00 |
| T18 | 7 | 9513 | 1.00 | 13950 | 0.29 | 3331 | 0.43 | 2896 | 0.43 | 3757 | 0.29 | 2505 | 1.00 |
| T19 | 18 | 34131 | 1.00 | 16247 | 0.22 | 3238 | 0.22 | 2800 | 0.22 | 3529 | 0.22 | 8433 | 1.00 |
| T20 | 21 | 27674 | 1.00 | 12526 | 0.14 | 3369 | 0.24 | 2931 | 0.24 | 3287 | 0.14 | 9569 | 1.00 |

## Command output compression (test_runner)

| Fixture | Raw tokens | Compressed tokens | Saved | Savings |
|---|---:|---:|---:|---:|
| cargo_test_no_run.raw.txt | 1688 | 833 | 855 | 50.7% |

## Caveats

- Tokens are estimates from a chars/words heuristic, not a model tokenizer.
- Ground truth: modified .rs files of each commit; added files excluded (undiscoverable before the change).
- Index input is the parent tree of each commit; the commit itself is never indexed.
- Recall@k for v4_get_task_context uses the files it returned as k.
- baseline_full_relevant and smart_read_skeleton_relevant use the ground truth and are references, not achievable variants.

## Goal normalization: German vs English goals

- Command: `cargo test --test v4_query_tests -- --ignored --nocapture goal_recall_report`
- Retrieval path: `SemanticContextAssembler::assemble` with default options (BM25 symbol ranking), index built on each task's parent snapshot, as in the suite above.
- Metric: Recall@5. Unique file paths in the assembler's output order, first 5, divided by the ground-truth file count of the task. Mean over the 20 tasks.
- German goals: `benches/token_benchmark/goals_de.json`, hand-written paraphrases of the 20 `tasks.json` goals. Ground truth unchanged.
- "German normalized" and "English normalized" run `query::normalize_goal` (terms plus identifiers) before matching; "raw" runs the goal string as before (`SemanticContextOptions::normalize_goal = false`).
- English normalized is an extra column; the three required numbers are German raw, German normalized and English raw.

| Variant | Mean Recall@5 | Tasks with Recall@5 > 0 |
|---|---:|---:|
| German raw | 0.146 | 7/20 |
| German normalized | 0.125 | 5/20 |
| English raw | 0.181 | 7/20 |
| English normalized (extra) | 0.177 | 6/20 |

Goal text size (token estimate, `token_stats::estimate_tokens`):

| Goal text | Total estimated tokens | Mean per goal |
|---|---:|---:|
| German goal (raw) | 384 | 19.2 |
| English goal (raw) | 314 | 15.7 |
| German goal, normalized query | 304 | 15.2 |

Per task (Recall@5):

| Task | German raw | German normalized | English raw | English normalized (extra) |
|---|---:|---:|---:|---:|
| T01 | 1.00 | 1.00 | 1.00 | 1.00 |
| T02 | 1.00 | 1.00 | 1.00 | 1.00 |
| T03 | 0.33 | 0.00 | 0.00 | 0.00 |
| T04 | 0.00 | 0.00 | 1.00 | 1.00 |
| T05 | 0.00 | 0.00 | 0.00 | 0.00 |
| T06 | 0.00 | 0.00 | 0.00 | 0.00 |
| T07 | 0.00 | 0.00 | 0.00 | 0.00 |
| T08 | 0.00 | 0.00 | 0.00 | 0.00 |
| T09 | 0.00 | 0.00 | 0.00 | 0.00 |
| T10 | 0.00 | 0.00 | 0.00 | 0.00 |
| T11 | 0.00 | 0.00 | 0.00 | 0.00 |
| T12 | 0.00 | 0.00 | 0.00 | 0.00 |
| T13 | 0.00 | 0.00 | 0.00 | 0.00 |
| T14 | 0.00 | 0.00 | 0.00 | 0.00 |
| T15 | 0.08 | 0.00 | 0.08 | 0.00 |
| T16 | 0.00 | 0.00 | 0.00 | 0.00 |
| T17 | 0.00 | 0.00 | 0.00 | 0.00 |
| T18 | 0.29 | 0.29 | 0.29 | 0.29 |
| T19 | 0.17 | 0.17 | 0.17 | 0.17 |
| T20 | 0.05 | 0.05 | 0.10 | 0.10 |

Reading of the result:

- The normalizer does not improve Recall@5 on this set. German normalized (0.125) is below German raw (0.146), and both are below English raw (0.181). The gap to English is not closed.
- Cause: vocabulary coverage. The 80-entry dictionary translates the common development words, but many goal words stay untranslated: `Semantik`, `Laufzeit`, `Gedächtnis`, `Zusammenfassungen`, `Probleme` (plural), `Verdrahtung`, and compounds such as `Kandidatensuche`, `Testkompilierfehler`, `Optionsinjektion`, `Pfadauflösung`. Untranslated words match no code symbol, so they only add noise to the BM25 query.
- Cause: the dictionary was written before the measurement, without tuning to it. The goals were also written by the same author, so the set is not an independent check. Adding entries after seeing these numbers would overfit the 20 tasks, so none were added.
- Per-task differences are often one file out of 13 or 15 relevant files. With 20 tasks, a difference of 0.02 in the mean is within noise.
- Identifier spelling is preserved, so tasks whose German goal names the identifier (T01, T02) match as well as the English goal.
- Token effect is as expected: the normalized German query is about 21% smaller than the raw German goal (304 vs 384 tokens) and about 3% smaller than the English goal (314 tokens).
- Existing quirk, not changed here: `split_identifiers` splits the acronym `IDs` into `i` and `ds`, so `Session-IDs` loses its meaning. English `ids` is unaffected.
- Not measured: `get_task_context`'s summary fallback (`CandidateDiscovery`, legacy substring ranking) and the Legacy ranking mode. Legacy is kept raw as a reference. The measurement covers only the BM25 symbol path.
