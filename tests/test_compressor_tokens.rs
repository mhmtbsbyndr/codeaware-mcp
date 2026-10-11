mod common;
use codeaware_mcp::compressor::{compiler_output, generic, test_output};
use codeaware_mcp::v4::tokens::estimate_tokens;

fn report(name: &str, raw: &str, compressed: &str) -> usize {
    let (r, c) = (estimate_tokens(raw), estimate_tokens(compressed));
    if std::env::var("SHOW").is_ok() { println!("----- {name}\n{compressed}\n-----"); }
    println!("COMPRESS {name}: raw_tokens={r} compressed_tokens={c} lines={}->{}",
        raw.lines().count(), compressed.lines().count());
    c
}

#[test]
fn compressor_fixture_token_report() {
    let single = common::read_fixture("sample_cargo_test_output.txt");
    report("cargo_test_single", &single, &test_output::compress(&single, 20));

    let multi = common::read_fixture("sample_cargo_test_multi_failure.txt");
    let multi_out = report("cargo_test_multi", &multi, &test_output::compress(&multi, 20));
    assert!(multi_out * 5 < estimate_tokens(&multi), "failure-first output must stay under 20% of raw");

    let build = common::read_fixture("sample_cargo_build_errors.txt");
    let build_out = report("cargo_build_errors", &build, &compiler_output::compress(&build, 30));
    assert!(build_out * 2 < estimate_tokens(&build), "compile output must stay under 50% of raw");

    let log = common::read_fixture("sample_repeated_log.txt");
    let log_out = report("repeated_log", &log, &generic::compress(&log, 30));
    assert!(log_out * 10 < estimate_tokens(&log), "repeated lines must collapse to under 10% of raw");
}
