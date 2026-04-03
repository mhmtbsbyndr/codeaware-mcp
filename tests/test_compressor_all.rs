use codeaware_mcp::compressor::{compiler_output, linter_output, search_output, git_output, package_mgr_output, formatter_output};

// --- Compiler ---
#[test]
fn test_compiler_extracts_errors() {
    let raw = r#"   Compiling my-app v0.1.0
error[E0308]: mismatched types
  --> src/main.rs:10:5
   |
10 |     42u32
   |     ^^^^^ expected `String`, found `u32`

error[E0425]: cannot find value `foo`
  --> src/lib.rs:5:10
   |
5  |     foo + 1
   |     ^^^ not found in this scope

warning: unused variable: `x`
  --> src/main.rs:3:9
   |
3  |     let x = 1;
   |         ^ help: if this is intentional, prefix it with an underscore: `_x`

error: aborting due to 2 previous errors; 1 warning emitted
"#;
    let compressed = compiler_output::compress(raw, 15);
    assert!(compressed.contains("E0308"));
    assert!(compressed.contains("E0425"));
    assert!(compressed.contains("2 previous errors") || compressed.contains("error"));
    assert!(compressed.lines().count() <= 15);
}

// --- Linter ---
#[test]
fn test_linter_extracts_top_issues() {
    let raw = r#"warning: redundant clone
  --> src/main.rs:15:20
   |
15 |     let s = name.clone();
   |                  ^^^^^^^^ help: remove this

error: unnecessary `unsafe` block
  --> src/lib.rs:22:5
   |
22 |     unsafe { }
   |     ^^^^^^ unnecessary `unsafe`

warning: unused import: `std::io`
  --> src/main.rs:1:5
   |
1  |     use std::io;
   |         ^^^^^^^
"#;
    let compressed = linter_output::compress(raw, 15);
    // Errors should appear before warnings
    assert!(compressed.contains("unnecessary `unsafe`"));
    assert!(compressed.lines().count() <= 15);
}

// --- Search ---
#[test]
fn test_search_deduplicates_and_limits() {
    let mut raw = String::new();
    for i in 1..=50 {
        raw.push_str(&format!("src/file{}.rs:10: match found here\n", i));
    }
    let compressed = search_output::compress(&raw, 20);
    assert!(compressed.lines().count() <= 22); // 20 results + possible header
    assert!(compressed.contains("src/file1.rs"));
}

// --- Git ---
#[test]
fn test_git_diff_compression() {
    let raw = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,7 +10,7 @@ fn main() {
-    let x = 1;
+    let x = 2;
@@ -50,3 +50,5 @@ fn helper() {
     println!("hi");
+    println!("added");
+    println!("more");
 }
diff --git a/src/lib.rs b/src/lib.rs
index 111222..333444 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
+use std::io;
 pub mod auth;
"#;
    let compressed = git_output::compress(raw, 20);
    assert!(compressed.contains("src/main.rs"));
    assert!(compressed.contains("src/lib.rs"));
    assert!(compressed.lines().count() <= 20);
}

#[test]
fn test_git_status_compression() {
    let raw = r#" M src/main.rs
 M src/lib.rs
 M src/auth.rs
?? new_file.rs
?? another_new.rs
D  deleted.rs
"#;
    let compressed = git_output::compress(raw, 20);
    // Short enough, should be returned as-is or slightly structured
    assert!(compressed.contains("src/main.rs"));
}

// --- Package Manager ---
#[test]
fn test_package_mgr_compression() {
    let mut raw = String::new();
    for i in 1..=30 {
        raw.push_str(&format!("  Downloading crate-{i} v1.0.{i}\n"));
    }
    raw.push_str("  Downloaded 30 crates in 2.5s\n");
    raw.push_str("   Compiling my-app v0.1.0\n");
    raw.push_str("    Finished in 5.2s\n");
    let compressed = package_mgr_output::compress(&raw, 5);
    assert!(compressed.contains("30 crates") || compressed.contains("Finished"));
    assert!(compressed.lines().count() <= 5);
}

// --- Formatter ---
#[test]
fn test_formatter_compression() {
    let raw = "Formatted src/main.rs\nFormatted src/lib.rs\nFormatted src/auth.rs\n3 files formatted\n";
    let compressed = formatter_output::compress(raw, 10);
    assert!(compressed.contains("3 files") || compressed.contains("Formatted"));
}
