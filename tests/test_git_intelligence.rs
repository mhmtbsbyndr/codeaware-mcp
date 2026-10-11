use std::process::Command;
use tempfile::TempDir;

/// Helper: create a temp git repo with some commits
fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let path = dir.path();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(path)
            .env("GIT_AUTHOR_NAME", "Test Author")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test Author")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .expect("git command failed");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["init"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test Author"]);

    // First commit
    std::fs::write(path.join("hello.rs"), "fn main() {\n    println!(\"hello\");\n}\n")
        .unwrap();
    run(&["add", "hello.rs"]);
    run(&["commit", "-m", "feat: initial hello"]);

    // Second commit
    std::fs::write(
        path.join("hello.rs"),
        "fn main() {\n    println!(\"hello world\");\n    println!(\"goodbye\");\n}\n",
    )
    .unwrap();
    std::fs::write(path.join("lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n")
        .unwrap();
    run(&["add", "hello.rs", "lib.rs"]);
    run(&["commit", "-m", "fix: update greeting and add lib"]);

    // Third commit
    std::fs::write(path.join("README.md"), "# Project\n").unwrap();
    run(&["add", "README.md"]);
    run(&["commit", "-m", "docs: add readme"]);

    dir
}

#[test]
fn test_git_diff_structured_output() {
    let repo = setup_git_repo();
    let path = repo.path();

    // Run git diff between HEAD~1 and HEAD (should show README.md added)
    let output = Command::new("git")
        .args(["diff", "--numstat", "HEAD~1", "HEAD"])
        .current_dir(path)
        .output()
        .expect("git diff failed");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("README.md"));
}

#[test]
fn test_git_diff_via_handler() {
    let repo = setup_git_repo();
    let cwd = repo.path().to_str().unwrap().to_string();

    let params = serde_json::json!({
        "base": "HEAD~1",
        "head": "HEAD",
        "cwd": cwd
    });

    let result = codeaware_mcp::tools::git_intelligence::handle_git_diff(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(envelope["ok"].as_bool().unwrap());
    assert!(envelope["data"]["files_changed"].as_u64().unwrap() >= 1);

    // Check that README.md is in the files list
    let files = envelope["data"]["files"].as_array().unwrap();
    let paths: Vec<&str> = files
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"README.md"));
}

#[test]
fn test_git_blame_via_handler() {
    let repo = setup_git_repo();
    let cwd = repo.path().to_str().unwrap().to_string();

    let params = serde_json::json!({
        "file": "hello.rs",
        "start_line": 1,
        "end_line": 3,
        "cwd": cwd
    });

    let result = codeaware_mcp::tools::git_intelligence::handle_git_blame(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(envelope["ok"].as_bool().unwrap());
    assert_eq!(envelope["data"]["file"].as_str().unwrap(), "hello.rs");

    let lines = envelope["data"]["lines"].as_array().unwrap();
    assert!(!lines.is_empty());
    // All lines should have Test Author
    for line in lines {
        assert_eq!(line["author"].as_str().unwrap(), "Test Author");
    }
}

#[test]
fn test_git_blame_missing_file_param() {
    let params = serde_json::json!({});
    let result = codeaware_mcp::tools::git_intelligence::handle_git_blame(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(!envelope["ok"].as_bool().unwrap());
    assert_eq!(envelope["error_code"].as_str().unwrap(), "E_GIT_ERROR");
}

#[test]
fn test_git_changelog_via_handler() {
    let repo = setup_git_repo();
    let cwd = repo.path().to_str().unwrap().to_string();

    let params = serde_json::json!({
        "limit": 10,
        "cwd": cwd
    });

    let result = codeaware_mcp::tools::git_intelligence::handle_git_changelog(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(envelope["ok"].as_bool().unwrap());

    let entries = envelope["data"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);

    // Check categories from conventional commits
    let categories: Vec<&str> = entries
        .iter()
        .map(|e| e["category"].as_str().unwrap())
        .collect();
    assert!(categories.contains(&"docs"));
    assert!(categories.contains(&"bugfix"));
    assert!(categories.contains(&"feature"));
}

#[test]
fn test_git_changelog_with_base_ref() {
    let repo = setup_git_repo();
    let cwd = repo.path().to_str().unwrap().to_string();

    // Only get last 2 commits
    let params = serde_json::json!({
        "base": "HEAD~2",
        "head": "HEAD",
        "cwd": cwd
    });

    let result = codeaware_mcp::tools::git_intelligence::handle_git_changelog(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(envelope["ok"].as_bool().unwrap());
    assert_eq!(envelope["data"]["total_commits"].as_u64().unwrap(), 2);
}

#[test]
fn test_git_diff_in_non_git_dir() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let cwd = dir.path().to_str().unwrap().to_string();

    let params = serde_json::json!({
        "cwd": cwd
    });
    let result = codeaware_mcp::tools::git_intelligence::handle_git_diff(&params);

    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(!envelope["ok"].as_bool().unwrap());
    assert_eq!(envelope["error_code"].as_str().unwrap(), "E_GIT_ERROR");
}

// ── Regression: option injection via user-supplied refs/paths ────

#[test]
fn test_git_diff_rejects_output_option_injection() {
    let repo = setup_git_repo();
    let target = repo.path().join("pwned_diff.txt");
    let base = format!("--output={}", target.display());

    let params = serde_json::json!({
        "cwd": repo.path().to_str().unwrap(),
        "base": base,
        "head": "HEAD"
    });
    let result = codeaware_mcp::tools::git_intelligence::handle_git_diff(&params);
    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(!envelope["ok"].as_bool().unwrap(), "option-like base must be rejected");
    assert!(!target.exists(), "git must not write the --output file");
}

#[test]
fn test_git_changelog_rejects_output_option_injection() {
    let repo = setup_git_repo();
    let target = repo.path().join("pwned_log.txt");
    let base = format!("--output={}", target.display());

    let params = serde_json::json!({
        "cwd": repo.path().to_str().unwrap(),
        "base": base,
        "head": "HEAD"
    });
    let result = codeaware_mcp::tools::git_intelligence::handle_git_changelog(&params);
    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(!envelope["ok"].as_bool().unwrap(), "option-like base must be rejected");
    assert!(!target.exists(), "git must not write the --output file");
}

#[test]
fn test_git_blame_rejects_output_option_injection() {
    let repo = setup_git_repo();
    let target = repo.path().join("pwned_blame.txt");
    let file = format!("--output={}", target.display());

    let params = serde_json::json!({
        "cwd": repo.path().to_str().unwrap(),
        "file": file
    });
    let result = codeaware_mcp::tools::git_intelligence::handle_git_blame(&params);
    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(!envelope["ok"].as_bool().unwrap(), "option-like file must be rejected");
    assert!(!target.exists(), "git must not write the --output file");
}

// ── Regression: porcelain blame must attribute each line to its own commit ──

#[test]
fn test_git_blame_attributes_repeated_commit_correctly() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path();

    let commit = |author: &str, content: &str, msg: &str| {
        std::fs::write(path.join("ab.txt"), content).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(path)
                .env("GIT_AUTHOR_NAME", author)
                .env("GIT_AUTHOR_EMAIL", "a@example.com")
                .env("GIT_COMMITTER_NAME", author)
                .env("GIT_COMMITTER_EMAIL", "a@example.com")
                .output()
                .expect("git");
            assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
        };
        run(&["add", "ab.txt"]);
        run(&["commit", "-m", msg]);
    };

    Command::new("git").args(["init"]).current_dir(path).output().unwrap();
    // Commit A (Alice): lines 1 and 3
    commit("Alice", "one\nthree\n", "feat: A");
    // Commit B (Bob): line 2 inserted between them
    commit("Bob", "one\ntwo\nthree\n", "feat: B");

    let params = serde_json::json!({
        "cwd": path.to_str().unwrap(),
        "file": "ab.txt"
    });
    let result = codeaware_mcp::tools::git_intelligence::handle_git_blame(&params);
    let text = result["content"][0]["text"].as_str().unwrap();
    let envelope: serde_json::Value = serde_json::from_str(text).unwrap();
    let lines = envelope["data"]["lines"].as_array().unwrap();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["author"], "Alice");
    assert_eq!(lines[1]["author"], "Bob");
    assert_eq!(lines[2]["author"], "Alice", "line 3 belongs to Alice's second commit");
}
