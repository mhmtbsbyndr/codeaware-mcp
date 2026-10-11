//! Scale checks: inputs up to ~100k estimated tokens must stay correct and bounded.
//!
//! Token numbers use `token_stats::estimate_tokens` (a heuristic, not a real
//! tokenizer). 100k tokens is roughly 400 KB of ASCII prose or code.

use codeaware_mcp::compressor::compress_output;
use codeaware_mcp::token_stats::estimate_tokens;
use codeaware_mcp::tools::smart_read::{smart_read, ReadMode, SmartReadInput};
use std::fs;
use std::time::Instant;

const TOKENS_100K_CHARS: usize = 400_000;

fn tokens(text: &str) -> u64 {
    estimate_tokens(text)
}

/// A large cargo test log: many passing lines, a few failures in the middle.
fn big_test_log(passing: usize) -> String {
    let mut s = String::from("running ".to_string().as_str());
    s.push_str(&format!("{passing} tests\n"));
    for i in 0..passing {
        s.push_str(&format!("test module::case_{i:06} ... ok\n"));
    }
    s.push_str("test module::broken_parser ... FAILED\n");
    s.push_str("\nfailures:\n\n---- module::broken_parser stdout ----\n");
    s.push_str("thread 'module::broken_parser' panicked at src/parser.rs:42:9:\n");
    s.push_str("assertion failed: left == right\n\n");
    s.push_str("test result: FAILED. 1 failed; 0 passed; 0 ignored\n");
    s
}

#[test]
fn test_log_near_100k_tokens_keeps_failure_and_summary() {
    let raw = big_test_log(12_000);
    let raw_tokens = tokens(&raw);
    assert!(raw_tokens >= 80_000, "fixture should be near 100k tokens, got {raw_tokens}");

    let started = Instant::now();
    let out = compress_output("test_runner", &raw, 60);
    let elapsed = started.elapsed();

    assert!(out.contains("module::broken_parser"), "failing test name must survive");
    assert!(out.contains("src/parser.rs:42"), "panic location must survive");
    assert!(out.contains("test result: FAILED"), "summary line must survive");
    assert!(
        tokens(&out) * 20 < raw_tokens,
        "compressed output should be under 5% of input ({} vs {raw_tokens})",
        tokens(&out)
    );
    assert!(elapsed.as_secs() < 5, "compression of ~100k tokens took {elapsed:?}");
}

#[test]
fn repeated_identical_log_lines_collapse_at_scale() {
    let mut raw = String::new();
    for _ in 0..20_000 {
        raw.push_str("WARN retrying connection to upstream, attempt failed\n");
    }
    raw.push_str("ERROR giving up after 20000 attempts\n");
    let out = compress_output("generic", &raw, 40);
    assert!(out.contains("ERROR giving up"), "the final error line must survive");
    assert!(tokens(&out) < 2_000, "collapsed output too large: {}", tokens(&out));
}

#[test]
fn smart_read_of_100k_token_file_is_bounded_in_skeleton_mode() {
    let dir = tempdir();
    let mut src = String::new();
    let mut i = 0usize;
    while src.len() < TOKENS_100K_CHARS {
        src.push_str(&format!(
            "/// Doc line for item {i}.\npub fn generated_item_{i}(x: u64) -> u64 {{\n    x + {i}\n}}\n\n"
        ));
        i += 1;
    }
    fs::write(dir.join("big.rs"), &src).unwrap();

    let input = SmartReadInput {
        path: "big.rs".into(),
        mode: ReadMode::Skeleton,
        focus: None,
        lines: None,
        scope: None,
    };
    let started = Instant::now();
    let result = smart_read(&input, &dir).expect("skeleton read must succeed");
    assert!(started.elapsed().as_secs() < 5);

    // Measure what the client receives: the serialized tool result.
    let rendered = serde_json::to_string(&result).expect("result must serialize");
    assert!(
        tokens(&rendered) < tokens(&src) / 4,
        "skeleton response must be far smaller than the file: {} vs {}",
        tokens(&rendered),
        tokens(&src)
    );
    assert!(result.loc > 1_000, "loc must reflect the full file");
}

#[test]
fn smart_read_refuses_files_above_size_cap_without_reading_them() {
    let dir = tempdir();
    let path = dir.join("huge.txt");
    let f = fs::File::create(&path).unwrap();
    // 9 MiB sparse file: above the 8 MiB cap, never fully materialised.
    f.set_len(9 * 1024 * 1024).unwrap();

    let input = SmartReadInput {
        path: "huge.txt".into(),
        mode: ReadMode::Full,
        focus: None,
        lines: None,
        scope: None,
    };
    let err = smart_read(&input, &dir).expect_err("oversized file must be refused");
    assert!(format!("{err:?}").to_lowercase().contains("large") || format!("{err:?}").contains("TooLarge"));
}

/// Minimal unique temp dir without extra crates.
fn tempdir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("codeaware-scale-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}
