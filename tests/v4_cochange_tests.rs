//! Co-change history from git (`codeaware_mcp::v4::cochange`) and leakage checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use codeaware_mcp::v4::cochange::{CoChangeHistory, COCHANGE_DEPTH};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit(dir: &Path, files: &[(&str, &str)], message: &str) -> String {
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
    git(dir, &["rev-parse", "HEAD"])
}

/// Repo with three commits. c1 and c2 touch a.rs+b.rs. c3 is the "task" commit touching a.rs+d.rs.
fn fixture_repo(name: &str) -> (PathBuf, String, String, String) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    let c1 = commit(&dir, &[("src/a.rs", "1"), ("src/b.rs", "1")], "c1");
    let c2 = commit(&dir, &[("src/a.rs", "2"), ("src/b.rs", "2")], "c2");
    let c3 = commit(&dir, &[("src/a.rs", "3"), ("src/d.rs", "3")], "c3 task");
    (dir, c1, c2, c3)
}

#[test]
fn history_from_parent_excludes_the_task_commit() {
    let (dir, c1, c2, c3) = fixture_repo("cochange-leak-parent");
    let history = CoChangeHistory::from_git(&dir, &c2, COCHANGE_DEPTH);
    assert_eq!(history.commit_count(), 2);
    assert!(history.contains_commit(&c1));
    assert!(history.contains_commit(&c2));
    assert!(!history.contains_commit(&c3), "the evaluated commit must not be read");

    let universe: std::collections::BTreeSet<String> =
        ["src/a.rs", "src/b.rs", "src/d.rs"].iter().map(|s| s.to_string()).collect();
    let partners = history.partners(&["src/a.rs"], &universe);
    assert!(partners.iter().all(|p| p.path != "src/d.rs"), "d.rs only changed in the task commit");
    assert!(partners.iter().any(|p| p.path == "src/b.rs"));
}

#[test]
fn history_from_head_contains_the_task_commit_so_the_parent_rule_matters() {
    let (dir, _c1, _c2, c3) = fixture_repo("cochange-leak-head");
    let history = CoChangeHistory::from_git(&dir, "HEAD", COCHANGE_DEPTH);
    assert_eq!(history.commit_count(), 3);
    assert!(history.contains_commit(&c3));
}

#[test]
fn depth_bounds_the_number_of_commits_read() {
    let (dir, _c1, _c2, _c3) = fixture_repo("cochange-depth");
    let history = CoChangeHistory::from_git(&dir, "HEAD", 1);
    assert_eq!(history.commit_count(), 1);
}

#[test]
fn missing_repository_and_unknown_revision_give_empty_history() {
    let (dir, _c1, _c2, _c3) = fixture_repo("cochange-unknown-rev");
    assert_eq!(CoChangeHistory::from_git(&dir, "no-such-revision-xyz", COCHANGE_DEPTH).commit_count(), 0);
    let not_a_repo = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cochange-not-a-repo");
    std::fs::create_dir_all(&not_a_repo).unwrap();
    assert_eq!(CoChangeHistory::from_git(&not_a_repo, "HEAD", COCHANGE_DEPTH).commit_count(), 0);
}

#[test]
fn revision_that_looks_like_an_option_is_refused() {
    let (dir, _c1, _c2, _c3) = fixture_repo("cochange-option");
    assert_eq!(CoChangeHistory::from_git(&dir, "--all", COCHANGE_DEPTH).commit_count(), 0);
}

#[test]
fn partner_scores_match_jaccard_on_real_history() {
    let (dir, _c1, c2, c3) = fixture_repo("cochange-jaccard");
    let parent = git(&dir, &["rev-parse", &format!("{c3}^")]);
    assert_eq!(parent, c2);
    let before = CoChangeHistory::from_git(&dir, &parent, COCHANGE_DEPTH);
    assert_eq!(before.commit_count(), 2);
    let universe: std::collections::BTreeSet<String> = ["src/a.rs", "src/b.rs"].iter().map(|s| s.to_string()).collect();
    let partners = before.partners(&["src/a.rs"], &universe);
    // a in 2 commits, b in 2 commits, together in 2: Jaccard 1.0.
    assert_eq!(partners.len(), 1);
    assert!((partners[0].score - 1.0).abs() < 1e-9);
}
