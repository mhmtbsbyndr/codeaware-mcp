//! Co-change signal from git history: files that were committed together.
//!
//! `CoChangeHistory::from_git(repo, rev, depth)` reads the last `depth` commits reachable from
//! `rev` with `git log --name-only`. Passing a parent commit as `rev` gives exactly the history
//! that precedes it, so a benchmark task never sees its own commit or anything after it.
//! Commits that touch more than `MAX_FILES_PER_COMMIT` files (releases, mass renames) are ignored,
//! because they link unrelated files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Number of commits read from history by default.
pub const COCHANGE_DEPTH: usize = 500;
/// Commits touching more files than this carry no co-change signal and are skipped.
pub const MAX_FILES_PER_COMMIT: usize = 30;

/// Record separator written before each commit hash, so file lists can be split per commit.
const RECORD_SEPARATOR: char = '\u{1e}';

/// A file that co-changed with one of the seed files.
#[derive(Debug, Clone, PartialEq)]
pub struct Partner {
    pub path: String,
    /// Sum over seeds of the Jaccard index of commits touching both files.
    pub score: f64,
    /// Seed with the highest Jaccard index for this partner.
    pub seed: String,
}

#[derive(Debug, Clone, Default)]
pub struct CoChangeHistory {
    commits: Vec<String>,
    file_counts: BTreeMap<String, usize>,
    /// Keys are `(a, b)` with `a < b`.
    pair_counts: BTreeMap<(String, String), usize>,
}

impl CoChangeHistory {
    /// Reads up to `depth` commits reachable from `rev` in `repo`. Any git failure (not a
    /// repository, unknown revision, git missing) gives an empty history, so callers degrade to
    /// the BM25 signal alone.
    pub fn from_git(repo: &Path, rev: &str, depth: usize) -> Self {
        if rev.is_empty() || rev.starts_with('-') || depth == 0 {
            return Self::default();
        }
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["log", "--name-only", "--format=%x1e%H", "-n"])
            .arg(depth.to_string())
            .arg(rev)
            .arg("--")
            .output();
        match output {
            Ok(out) if out.status.success() => Self::from_log(&String::from_utf8_lossy(&out.stdout)),
            _ => Self::default(),
        }
    }

    /// Parses output of `git log --name-only --format=%x1e%H`.
    pub fn from_log(log: &str) -> Self {
        let mut history = Self::default();
        for record in log.split(RECORD_SEPARATOR) {
            let mut lines = record.lines().map(str::trim).filter(|line| !line.is_empty());
            let Some(hash) = lines.next() else {
                continue;
            };
            let files: BTreeSet<String> = lines.map(str::to_string).collect();
            history.commits.push(hash.to_string());
            if files.is_empty() || files.len() > MAX_FILES_PER_COMMIT {
                continue;
            }
            for file in &files {
                *history.file_counts.entry(file.clone()).or_insert(0) += 1;
            }
            let ordered: Vec<&String> = files.iter().collect();
            for (i, a) in ordered.iter().enumerate() {
                for b in &ordered[i + 1..] {
                    *history
                        .pair_counts
                        .entry(((*a).clone(), (*b).clone()))
                        .or_insert(0) += 1;
                }
            }
        }
        history
    }

    /// Commits read, including ones that were skipped for their size.
    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    /// True when `hash` (full or prefix) is one of the commits read.
    pub fn contains_commit(&self, hash: &str) -> bool {
        !hash.is_empty() && self.commits.iter().any(|commit| commit.starts_with(hash))
    }

    /// Files in `universe` that co-changed with the seeds, best first. A partner's score is the
    /// sum over seeds of `|commits(seed) & commits(partner)| / |commits(seed) | commits(partner)|`.
    /// Seeds are not returned as their own partners.
    pub fn partners(&self, seeds: &[&str], universe: &BTreeSet<String>) -> Vec<Partner> {
        // partner -> (summed score, best single jaccard, seed that gave it)
        let mut acc: BTreeMap<String, (f64, f64, String)> = BTreeMap::new();
        for &seed in seeds {
            let Some(&seed_count) = self.file_counts.get(seed) else {
                continue;
            };
            for ((a, b), &together) in &self.pair_counts {
                let partner = if a == seed {
                    b
                } else if b == seed {
                    a
                } else {
                    continue;
                };
                if !universe.contains(partner) {
                    continue;
                }
                let partner_count = self.file_counts.get(partner).copied().unwrap_or(0);
                let union = seed_count + partner_count - together;
                let jaccard = together as f64 / union as f64;
                let slot = acc.entry(partner.clone()).or_insert((0.0, 0.0, seed.to_string()));
                slot.0 += jaccard;
                if jaccard > slot.1 {
                    slot.1 = jaccard;
                    slot.2 = seed.to_string();
                }
            }
        }
        let mut out: Vec<Partner> = acc
            .into_iter()
            .filter(|(_, (score, _, _))| *score > 0.0)
            .map(|(path, (score, _, seed))| Partner { path, score, seed })
            .collect();
        out.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(records: &[(&str, &[&str])]) -> String {
        records
            .iter()
            .map(|(hash, files)| format!("{RECORD_SEPARATOR}{hash}\n\n{}\n", files.join("\n")))
            .collect()
    }

    #[test]
    fn parses_commits_and_counts_pairs() {
        let history = CoChangeHistory::from_log(&log(&[
            ("aaa111", &["src/a.rs", "src/b.rs"]),
            ("bbb222", &["src/a.rs"]),
        ]));
        assert_eq!(history.commit_count(), 2);
        assert_eq!(history.file_counts["src/a.rs"], 2);
        assert_eq!(history.pair_counts[&("src/a.rs".to_string(), "src/b.rs".to_string())], 1);
    }

    #[test]
    fn skips_oversized_commits_for_pairs_but_still_counts_them() {
        let many: Vec<String> = (0..=MAX_FILES_PER_COMMIT).map(|i| format!("src/f{i}.rs")).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let history = CoChangeHistory::from_log(&log(&[("big", &refs)]));
        assert_eq!(history.commit_count(), 1);
        assert!(history.file_counts.is_empty());
        assert!(history.pair_counts.is_empty());
    }

    #[test]
    fn partners_use_jaccard_and_respect_universe() {
        // a+b together twice, a alone once, a+c once.
        let history = CoChangeHistory::from_log(&log(&[
            ("1", &["src/a.rs", "src/b.rs"]),
            ("2", &["src/a.rs", "src/b.rs"]),
            ("3", &["src/a.rs"]),
            ("4", &["src/a.rs", "src/c.rs"]),
        ]));
        let universe: BTreeSet<String> = ["src/a.rs", "src/b.rs"].iter().map(|s| s.to_string()).collect();
        let partners = history.partners(&["src/a.rs"], &universe);
        assert_eq!(partners.len(), 1, "c.rs is outside the universe");
        assert_eq!(partners[0].path, "src/b.rs");
        // a: 4 commits, b: 2, together 2 -> Jaccard 2/4.
        assert!((partners[0].score - 0.5).abs() < 1e-9);
        assert_eq!(partners[0].seed, "src/a.rs");
    }

    #[test]
    fn seed_is_never_its_own_partner_and_unknown_seed_gives_nothing() {
        let history = CoChangeHistory::from_log(&log(&[("1", &["src/a.rs", "src/b.rs"])]));
        let universe: BTreeSet<String> = ["src/a.rs", "src/b.rs"].iter().map(|s| s.to_string()).collect();
        assert!(history.partners(&["src/unknown.rs"], &universe).iter().all(|p| p.path != "src/unknown.rs"));
        let from_a = history.partners(&["src/a.rs"], &universe);
        assert!(from_a.iter().all(|p| p.path != "src/a.rs"));
    }

    #[test]
    fn invalid_or_missing_repository_gives_empty_history() {
        let dir = std::env::temp_dir().join("codeaware-cochange-missing-repo-unit");
        let history = CoChangeHistory::from_git(&dir, "HEAD", COCHANGE_DEPTH);
        assert_eq!(history.commit_count(), 0);
        let flag_like = CoChangeHistory::from_git(Path::new("."), "--output=/etc/x", COCHANGE_DEPTH);
        assert_eq!(flag_like.commit_count(), 0);
    }
}
