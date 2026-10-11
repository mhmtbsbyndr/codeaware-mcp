//! Compression must never drop failing names, panic lines, error messages or file:line locations.
mod common;
use codeaware_mcp::compressor::{compiler_output, compress_output, generic, linter_output, test_output};

const FAILING: [&str; 3] = [
    "cache::tests::test_eviction_policy",
    "queue::tests::test_retry_backoff",
    "integration::test_error_recovery",
];
const PANIC_LOCATIONS: [&str; 3] = [
    "src/cache/tests.rs:88:9",
    "src/queue/tests.rs:142:5",
    "src/integration_tests.rs:234:5",
];

#[test]
fn cargo_failures_survive_every_budget() {
    let raw = common::read_fixture("sample_cargo_test_multi_failure.txt");
    for budget in [3, 6, 8, 20] {
        let out = test_output::compress(&raw, budget);
        for name in FAILING {
            assert!(out.contains(name), "budget {budget}: failing test {name} dropped:\n{out}");
        }
        for loc in PANIC_LOCATIONS {
            assert!(out.contains(loc), "budget {budget}: panic location {loc} dropped:\n{out}");
        }
        assert!(out.contains("57 passed; 3 failed"), "budget {budget}: summary counts dropped");
        assert!(!out.contains("test cache::tests::test_eviction_policy ... ok"));
    }
}

#[test]
fn cargo_failures_include_every_test_binary_summary() {
    let raw = "running 2 tests\ntest a ... ok\ntest b ... FAILED\n\nfailures:\n\n---- b stdout ----\nthread 'b' panicked at src/b.rs:3:1:\nboom\n\nfailures:\n    b\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored\n\n     Running tests/doc.rs\n\ntest result: ok. 4 passed; 0 failed; 0 ignored\n";
    let out = test_output::compress(raw, 4);
    assert!(out.contains("1 passed; 1 failed"), "{out}");
    assert!(out.contains("4 passed; 0 failed"), "{out}");
    assert!(out.contains("src/b.rs:3:1"), "{out}");
}

#[test]
fn pytest_failures_survive_tight_budget() {
    let mut raw = String::from("============================= test session starts ==============================\ncollected 30 items\n\n");
    for i in 0..25 {
        raw.push_str(&format!("tests/t.py::test_ok_{i} PASSED\n"));
    }
    raw.push_str("FAILED tests/a.py::test_one - AssertionError: expected 401\n");
    raw.push_str("FAILED tests/b.py::test_two - KeyError: 'token'\n");
    raw.push_str("============================== 2 failed, 25 passed ==============================\n");
    let out = test_output::compress(&raw, 4);
    assert!(out.contains("test_one") && out.contains("test_two"), "{out}");
    assert!(out.contains("expected 401") && out.contains("KeyError"), "{out}");
}

#[test]
fn compile_failure_under_test_runner_keeps_all_errors() {
    let raw = common::read_fixture("sample_cargo_build_errors.txt");
    let out = test_output::compress(&raw, 3);
    for code in ["E0308", "E0425", "E0599"] {
        assert!(out.contains(code), "error {code} dropped:\n{out}");
    }
    assert!(out.contains("src/db/pool.rs:140:10"), "{out}");
}

#[test]
fn compiler_keeps_every_error_and_location_under_tiny_budget() {
    let raw = common::read_fixture("sample_cargo_build_errors.txt");
    let out = compiler_output::compress(&raw, 2);
    for loc in ["src/handlers/user.rs:87:24", "src/auth/session.rs:12:18", "src/db/pool.rs:140:10"] {
        assert!(out.contains(loc), "location {loc} dropped:\n{out}");
    }
    assert!(out.contains("3 previous errors"), "{out}");
    assert!(out.contains("more warnings omitted"), "{out}");
}

#[test]
fn compiler_keeps_first_note_per_error_only() {
    let raw = common::read_fixture("sample_cargo_build_errors.txt");
    let out = compiler_output::compress(&raw, 40);
    assert!(out.contains("expected type `usize`"), "{out}");
    assert!(out.contains("did you mean `SESSION_TTL`"), "{out}");
    assert!(out.contains("the method `flush` exists"), "first note of each error is kept:\n{out}");
    assert!(!out.contains("consider importing"), "help lines are not notes:\n{out}");
    assert!(!out.contains("^^^"), "code snippets must be dropped:\n{out}");

    let two_notes = "error[E0001]: boom\n  --> src/x.rs:1:1\n   |\n   = note: first note\n   = note: second note\n\nerror: aborting due to 1 previous error\n";
    let out = compiler_output::compress(two_notes, 3);
    assert!(out.contains("first note") && !out.contains("second note"), "{out}");
}

#[test]
fn gcc_style_errors_keep_file_line_and_message() {
    let mut raw = String::new();
    for i in 0..10 {
        raw.push_str(&format!("src/unused{i}.c:4:2: warning: unused variable 'v{i}'\n"));
    }
    raw.push_str("src/main.c:12:5: error: 'foo' undeclared (first use in this function)\n");
    raw.push_str("src/main.c:20:1: fatal error: missing.h: No such file or directory\n");
    let out = compiler_output::compress(&raw, 4);
    assert!(out.contains("src/main.c:12:5: error: 'foo' undeclared"), "{out}");
    assert!(out.contains("src/main.c:20:1: fatal error: missing.h"), "{out}");
}

#[test]
fn linter_never_drops_errors_when_warnings_exceed_budget() {
    let mut raw = String::new();
    for i in 0..10 {
        raw.push_str(&format!("warning: style nit {i}\n  --> src/w.rs:{}:1\n\n", i + 1));
    }
    raw.push_str("error: unnecessary `unsafe` block\n  --> src/lib.rs:22:5\n");
    let out = linter_output::compress(&raw, 3);
    assert!(out.contains("src/lib.rs:22:5"), "{out}");
    assert!(out.contains("unnecessary `unsafe`"), "{out}");
}

#[test]
fn generic_keeps_error_lines_in_omitted_middle() {
    let raw = common::read_fixture("sample_repeated_log.txt");
    for budget in [3, 6, 10] {
        let out = generic::compress(&raw, budget);
        assert!(out.contains("connection refused at 10.0.0.5:5432"), "budget {budget}:\n{out}");
        assert!(out.contains("src/api/client.rs:211: upstream timeout"), "budget {budget}:\n{out}");
    }
}

#[test]
fn generic_repeated_log_collapses_with_example_and_count() {
    let raw = common::read_fixture("sample_repeated_log.txt");
    let out = generic::compress(&raw, 30);
    assert!(out.contains("heartbeat ok latency=10ms (x 40)"), "{out}");
    assert!(out.contains("GET /health 200 (cached) (x 15)"), "{out}");
    assert!(out.lines().count() < 10, "{out}");
}

#[test]
fn numeric_only_output_is_not_collapsed() {
    let raw: String = (1..=200).map(|i| format!("{i}\n")).collect();
    let out = generic::compress(&raw, 50);
    assert!(out.contains("200"), "{out}");
    assert!(!out.contains("(x "), "numeric lines must not merge:\n{out}");
}

#[test]
fn smart_run_routes_test_fixture_with_failures_intact() {
    let raw = common::read_fixture("sample_cargo_test_multi_failure.txt");
    let out = compress_output("test_runner", &raw, 20);
    for name in FAILING {
        assert!(out.contains(name), "{out}");
    }
}
