//! Goal normalizer tests (`codeaware_mcp::v4::query`) and the German goal recall measurement.
//!
//! The measurement is ignored by default because it materializes 20 git snapshots:
//! `cargo test --test v4_query_tests -- --ignored --nocapture goal_recall_report`

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use codeaware_mcp::token_benchmark::{load_tasks, materialize_snapshot};
use codeaware_mcp::token_stats::estimate_tokens;
use codeaware_mcp::v4::index_builder::SemanticIndexBuilder;
use codeaware_mcp::v4::query::{normalize_goal, DE_EN, MAX_IDENTIFIERS, MAX_TERMS};
use codeaware_mcp::v4::semantic_context::{SemanticContextAssembler, SemanticContextOptions};
use serde::Deserialize;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn terms(goal: &str) -> Vec<String> {
    normalize_goal(goal).terms
}

fn identifier_set(goal: &str) -> BTreeSet<String> {
    normalize_goal(goal).identifiers.into_iter().collect()
}

#[test]
fn german_and_english_goal_share_identifier_set() {
    let en = "Make get_task_context return SemanticContextAssembler results for src/v4/tools.rs";
    let de = "get_task_context so ändern, dass SemanticContextAssembler-Ergebnisse für src/v4/tools.rs zurückkommen";
    let expected: BTreeSet<String> = ["get_task_context", "SemanticContextAssembler", "src/v4/tools.rs"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(identifier_set(en), expected);
    assert_eq!(identifier_set(de), expected);
}

#[test]
fn german_and_english_goal_share_terms_through_dictionary() {
    let en = "Fix the error in the file";
    let de = "Fehler in der Datei beheben";
    assert_eq!(terms(en), vec!["error", "file"]);
    assert_eq!(terms(de), terms(en));
}

#[test]
fn identifiers_keep_original_spelling() {
    let goal = "getTaskContext in Foo::bar, src/v4/tools.rs and parse_goal_terms";
    let normalized = normalize_goal(goal);
    assert_eq!(
        normalized.identifiers,
        vec!["getTaskContext", "Foo::bar", "src/v4/tools.rs", "parse_goal_terms"]
    );
    assert!(normalized.keep_exact());
    // Terms are the identifier-split, lowercased parts.
    assert!(normalized.terms.contains(&"task".to_string()));
    assert!(normalized.terms.contains(&"goal".to_string()));
}

#[test]
fn plain_prose_has_no_identifiers() {
    let normalized = normalize_goal("Fehler in der Datei beheben");
    assert!(normalized.identifiers.is_empty());
    assert!(!normalized.keep_exact());
}

#[test]
fn umlauts_and_diacritics_are_handled() {
    // Umlaut spelling variants fold to the same dictionary key; unknown words keep their spelling.
    assert_eq!(terms("Abhängigkeit"), vec!["dependency"]);
    assert_eq!(terms("Abhaengigkeit"), vec!["dependency"]);
    assert_eq!(terms("Rückgabewert"), vec!["return", "value"]);
    assert_eq!(terms("Rueckgabewert"), vec!["return", "value"]);
    assert_eq!(terms("Änderungen an Größe"), vec!["change", "größe"]);
    assert_eq!(terms("Schließen"), vec!["schließen"]);
    // Accents do not break tokenization.
    assert_eq!(terms("Café-Modul"), vec!["café", "module"]);
}

#[test]
fn stopwords_are_removed_in_both_languages() {
    assert_eq!(terms("Please make the function use the cache"), vec!["function", "cache"]);
    assert_eq!(terms("Bitte mach die Funktion und den Cache"), vec!["function", "cache"]);
    assert!(terms("the and of und der die das").is_empty());
}

#[test]
fn empty_and_symbol_only_goals_return_no_terms() {
    for goal in ["", "   ", "?!", "--", "\n\t"] {
        let normalized = normalize_goal(goal);
        assert!(normalized.terms.is_empty(), "goal {goal:?}");
        assert!(normalized.identifiers.is_empty(), "goal {goal:?}");
        assert!(!normalized.keep_exact(), "goal {goal:?}");
        assert_eq!(normalized.search_query(), "", "goal {goal:?}");
    }
}

#[test]
fn very_long_goal_is_capped() {
    let letters = |mut n: usize| {
        let mut s = String::new();
        loop {
            s.push((b'a' + (n % 26) as u8) as char);
            n /= 26;
            if n == 0 {
                break;
            }
        }
        s
    };
    let goal: String = (0..300)
        .map(|i| format!("zz{} zz_{}", letters(i), letters(i)))
        .collect::<Vec<_>>()
        .join(" ");
    let normalized = normalize_goal(&goal);
    assert_eq!(normalized.terms.len(), MAX_TERMS);
    assert_eq!(normalized.identifiers.len(), MAX_IDENTIFIERS);
    assert_eq!(MAX_TERMS, 64);
}

#[test]
fn dictionary_entries_translate_to_their_english_values() {
    for (german, english) in DE_EN {
        let expected: Vec<String> = {
            let mut seen: Vec<String> = Vec::new();
            for word in english.split_whitespace() {
                if !seen.iter().any(|w| w == word) {
                    seen.push(word.to_string());
                }
            }
            seen
        };
        assert_eq!(terms(german), expected, "dictionary entry {german}");
    }
    assert!((70..=90).contains(&DE_EN.len()), "dictionary has {} entries", DE_EN.len());
}

#[test]
fn default_options_normalize_the_goal() {
    assert!(SemanticContextOptions::default().normalize_goal);
}

#[derive(Deserialize)]
struct GoalFile {
    goals: Vec<GoalEntry>,
}

#[derive(Deserialize)]
struct GoalEntry {
    id: String,
    goal: String,
}

fn load_goals(path: &std::path::Path) -> HashMap<String, String> {
    let text = std::fs::read_to_string(path).expect("goal file readable");
    let file: GoalFile = serde_json::from_str(&text).expect("goal file parses");
    file.goals.into_iter().map(|g| (g.id, g.goal)).collect()
}

#[test]
fn german_goal_file_covers_every_task() {
    let root = manifest_dir();
    let tasks = load_tasks(&root.join("benches/token_benchmark/tasks.json")).expect("tasks load");
    let de = load_goals(&root.join("benches/token_benchmark/goals_de.json"));
    assert_eq!(de.len(), 20);
    for task in &tasks {
        let goal = de.get(&task.id).unwrap_or_else(|| panic!("missing German goal for {}", task.id));
        assert!(!goal.trim().is_empty());
        assert_ne!(goal, &task.goal, "German goal {} must not be the English goal", task.id);
    }
}

/// Recall@5 of the BM25 symbol path (`SemanticContextAssembler`, the `v4_semantic_only`
/// variant) over the 20 task snapshots, for German raw, German normalized and English raw.
/// English normalized is an extra column. Prints a Markdown table and writes JSON to
/// CARGO_TARGET_TMPDIR.
#[test]
#[ignore = "measurement: materializes 20 git snapshots; run with --ignored --nocapture"]
fn goal_recall_report() {
    const K: usize = 5;
    let root = manifest_dir();
    let tasks = load_tasks(&root.join("benches/token_benchmark/tasks.json")).expect("tasks load");
    let de = load_goals(&root.join("benches/token_benchmark/goals_de.json"));
    let en: HashMap<String, String> = tasks.iter().map(|t| (t.id.clone(), t.goal.clone())).collect();
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("goal-recall");

    // (name, german goal?, normalize?)
    let variants: [(&str, bool, bool); 4] = [
        ("German raw", true, false),
        ("German normalized", true, true),
        ("English raw", false, false),
        ("English normalized (extra)", false, true),
    ];

    // recalls[variant][task]
    let mut recalls: Vec<Vec<f64>> = vec![Vec::new(); variants.len()];
    let mut rows: Vec<String> = Vec::new();
    for task in &tasks {
        let snapshot = work.join(&task.id);
        materialize_snapshot(&root, &task.parent, &snapshot).expect("snapshot materializes");
        let index = SemanticIndexBuilder::build(&snapshot).expect("index builds");
        let relevant: BTreeSet<&str> = task.relevant_files.iter().map(String::as_str).collect();

        let mut cells = Vec::new();
        for (v, (_, use_de, normalize)) in variants.iter().enumerate() {
            let goal = if *use_de { &de[&task.id] } else { &en[&task.id] };
            let options = SemanticContextOptions {
                normalize_goal: *normalize,
                ..Default::default()
            };
            let items = SemanticContextAssembler::assemble(goal, &index, options);
            let mut top: Vec<String> = Vec::new();
            for path in items.iter().filter_map(|item| item.path.clone()) {
                if !top.contains(&path) {
                    top.push(path);
                    if top.len() == K {
                        break;
                    }
                }
            }
            let hits = top.iter().filter(|p| relevant.contains(p.as_str())).count();
            let recall = if relevant.is_empty() { 0.0 } else { hits as f64 / relevant.len() as f64 };
            recalls[v].push(recall);
            cells.push(format!("{recall:.2}"));
        }
        rows.push(format!("| {} | {} | {} | {} | {} |", task.id, cells[0], cells[1], cells[2], cells[3]));
    }

    let mut report = String::new();
    report.push_str("| Task | German raw | German normalized | English raw | English normalized (extra) |\n");
    report.push_str("|---|---:|---:|---:|---:|\n");
    report.push_str(&rows.join("\n"));
    report.push('\n');
    report.push_str("\n| Variant | Mean Recall@5 | Tasks with Recall@5 > 0 |\n|---|---:|---:|\n");
    for (v, (name, _, _)) in variants.iter().enumerate() {
        let mean = recalls[v].iter().sum::<f64>() / recalls[v].len() as f64;
        let positive = recalls[v].iter().filter(|r| **r > 0.0).count();
        report.push_str(&format!("| {name} | {mean:.3} | {positive}/{} |\n", recalls[v].len()));
    }

    // Goal text size: German paraphrases vs the English goals, and the normalized query.
    let de_tokens: u64 = tasks.iter().map(|t| estimate_tokens(&de[&t.id])).sum();
    let en_tokens: u64 = tasks.iter().map(|t| estimate_tokens(&en[&t.id])).sum();
    let query_tokens: u64 = tasks
        .iter()
        .map(|t| estimate_tokens(&normalize_goal(&de[&t.id]).search_query()))
        .sum();
    report.push_str("\n| Goal text | Total estimated tokens | Mean per goal |\n|---|---:|---:|\n");
    for (name, total) in [
        ("German goal (raw)", de_tokens),
        ("English goal (raw)", en_tokens),
        ("German goal, normalized query", query_tokens),
    ] {
        report.push_str(&format!("| {name} | {total} | {:.1} |\n", total as f64 / tasks.len() as f64));
    }

    println!("{report}");
    std::fs::create_dir_all(&work).ok();
    std::fs::write(work.join("goal_recall_report.md"), &report).expect("report written");
}
