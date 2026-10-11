//! Token benchmark report tests.
//!
//! `quick_subset_produces_report` runs the pipeline on two tasks and writes to CARGO_TARGET_TMPDIR,
//! so it never touches committed results. `full_suite_writes_results` is ignored by default; run it
//! with `cargo test --test test_benchmark_report -- --ignored full_suite` to regenerate
//! benches/RESULTS.md and benches/RESULTS.json.

use codeaware_mcp::token_benchmark::{
    heuristic_counter, load_tasks, run_suite, write_report, SuiteConfig, DEFAULT_TOKEN_BUDGET,
    DEFAULT_TOP_K,
};
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn config(work_name: &str) -> SuiteConfig {
    SuiteConfig {
        repo: manifest_dir(),
        work_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(work_name),
        command_fixture: manifest_dir().join("benches/fixtures/token_benchmark/cargo_test_no_run.raw.txt"),
        top_k: DEFAULT_TOP_K,
        budget: DEFAULT_TOKEN_BUDGET,
        counter: heuristic_counter,
    }
}

#[test]
fn quick_subset_produces_report() {
    let tasks = load_tasks(&manifest_dir().join("benches/token_benchmark/tasks.json"))
        .expect("tasks.json must load");
    assert_eq!(tasks.len(), 20, "suite should contain 20 git-derived tasks");

    let subset: Vec<_> = tasks.iter().filter(|t| t.id == "T01" || t.id == "T05").cloned().collect();
    assert_eq!(subset.len(), 2);

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("token-bench-quick");
    let report = run_suite(&config("token-bench-quick-work"), &subset)
        .expect("suite should run");

    // 6 variants per task, one command-output row.
    assert_eq!(report.rows.len(), subset.len() * 6);
    assert_eq!(report.command_output.len(), 1);
    assert!(report.command_output[0].raw_tokens > 0);

    for row in &report.rows {
        assert!(row.tokens > 0, "{} {} produced no tokens", row.task_id, row.variant);
        assert!((0.0..=1.0).contains(&row.recall));
    }
    // The reference baseline reads the ground-truth files, so it always covers them.
    for row in report.rows.iter().filter(|r| r.variant == "baseline_full_relevant") {
        assert_eq!(row.recall, 1.0, "{} baseline must cover its relevant files", row.task_id);
    }

    let (md, json) = write_report(&report, &out_dir).expect("report should be written");
    let markdown = std::fs::read_to_string(&md).expect("RESULTS.md exists");
    assert!(markdown.contains("| Variant |"), "markdown summary table missing");
    assert!(markdown.contains("ESTIMATE ONLY"), "estimator caveat missing");
    let parsed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json).expect("RESULTS.json exists"))
            .expect("RESULTS.json is valid JSON");
    assert_eq!(parsed["task_count"], 2);
    assert!(parsed["summaries"].as_array().map_or(false, |s| !s.is_empty()));
}

#[test]
#[ignore = "full run; regenerates benches/RESULTS.md and benches/RESULTS.json"]
fn full_suite_writes_results() {
    let tasks = load_tasks(&manifest_dir().join("benches/token_benchmark/tasks.json"))
        .expect("tasks.json must load");
    let cfg = SuiteConfig {
        work_dir: manifest_dir().join("target/token-bench"),
        ..config("unused")
    };
    let report = run_suite(&cfg, &tasks).expect("suite should run");
    let (md, json) = write_report(&report, &manifest_dir().join("benches")).expect("report written");
    assert!(md.ends_with("benches/RESULTS.md"));
    assert!(json.ends_with("benches/RESULTS.json"));
    assert_eq!(report.task_count, 20);
}
