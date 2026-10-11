use std::fs;
use std::path::Path;

use codeaware_mcp::v4::{
    CallGraph, CodeSymbol, ContextItemKind, DiffImpactRequest, DiscoveryConfig,
    HeuristicCallExtractor, RustImportExtractor, RustTestExtractor, SemanticContextAssembler,
    SemanticContextOptions, SemanticIndex, SemanticTools, SummaryGenerator, SymbolIndex, SymbolKind,
    TaskIntent, TestGraph, V4Storage, V4Tools, CandidateDiscovery, GetTaskContextRequest,
};
use tempfile::tempdir;

fn write(root: &Path, rel: &str, content: &str) {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full, content).unwrap();
}

fn discover(root: &Path, goal: &str, max: usize) -> Vec<String> {
    let config = DiscoveryConfig {
        max_candidates: max,
        ..Default::default()
    };
    CandidateDiscovery::discover_ranked(root, goal, config)
        .unwrap()
        .into_iter()
        .map(|r| r.path)
        .collect()
}

// ---- item 1: discovery ----

#[test]
fn discovery_respects_gitignore() {
    let dir = tempdir().unwrap();
    write(dir.path(), ".gitignore", "generated/\n");
    write(dir.path(), "generated/x.rs", "pub fn generated_thing() {}");
    write(dir.path(), "src/keep.rs", "pub fn keep() {}");

    let paths = discover(dir.path(), "unrelated", 64);
    assert!(paths.iter().any(|p| p == "src/keep.rs"), "{paths:?}");
    assert!(!paths.iter().any(|p| p.starts_with("generated/")), "{paths:?}");
}

#[cfg(unix)]
#[test]
fn discovery_does_not_follow_symlinks_out_of_repo() {
    let outside = tempdir().unwrap();
    write(outside.path(), "secret.rs", "pub fn secret() {}");

    let dir = tempdir().unwrap();
    write(dir.path(), "src/keep.rs", "pub fn keep() {}");
    std::os::unix::fs::symlink(outside.path(), dir.path().join("linked")).unwrap();

    let paths = discover(dir.path(), "unrelated", 64);
    assert!(!paths.iter().any(|p| p.contains("secret")), "{paths:?}");
    assert!(paths.iter().any(|p| p == "src/keep.rs"), "{paths:?}");
}

#[test]
fn discovery_cap_applies_after_source_filter() {
    let dir = tempdir().unwrap();
    for i in 0..100 {
        write(dir.path(), &format!("docs/note_{i:03}.md"), "# note");
    }
    write(dir.path(), "zzz/lib.rs", "pub fn lib() {}");

    let paths = discover(dir.path(), "unrelated", 5);
    assert!(!paths.is_empty());
    assert!(paths.iter().all(|p| p.ends_with(".rs")), "{paths:?}");
    assert!(paths.iter().any(|p| p == "zzz/lib.rs"), "{paths:?}");
}

#[test]
fn discovery_order_is_deterministic() {
    let dir = tempdir().unwrap();
    write(dir.path(), "b.rs", "");
    write(dir.path(), "c.rs", "");
    write(dir.path(), "a.rs", "");

    let paths = discover(dir.path(), "unrelated", 2);
    assert_eq!(paths, vec!["a.rs".to_string(), "b.rs".to_string()]);
}

// ---- item 4: empty queries ----

#[test]
fn empty_queries_return_nothing() {
    let mut symbols = SymbolIndex::default();
    symbols.add(CodeSymbol {
        name: "anything".into(),
        kind: SymbolKind::Function,
        path: "src/a.rs".into(),
        line_start: 1,
        line_end: 1,
        signature: None,
    });
    assert!(symbols.find_by_name("").is_empty());
    assert!(symbols.find_by_name("   ").is_empty());

    let mut calls = CallGraph::default();
    calls.add("caller", "callee", "src/a.rs");
    assert!(calls.callers_of("").is_empty());
    assert!(calls.calls_from("").is_empty());

    let mut tests = TestGraph::default();
    tests.add("tests/a.rs", "test_thing");
    assert!(tests.find_tests_for_symbol("").is_empty());
}

// ---- item 5: call detection word boundaries ----

#[test]
fn call_edges_require_exact_identifier_match() {
    let source = "fn total(x: u32) -> u32 { x }\n\
                  fn caller() {\n    xtotal(3);\n    total(2);\n}\n\
                  fn other() {\n    run_all();\n}\n";
    let graph = HeuristicCallExtractor::extract("src/lib.rs", source);

    let callers = graph.callers_of("total");
    assert_eq!(callers.len(), 1, "{callers:?}");
    assert_eq!(callers[0].from_symbol, "caller");

    assert!(graph.callers_of("run").is_empty());
    assert_eq!(graph.callers_of("run_all").len(), 1);
    assert!(graph.edges.iter().any(|e| e.from_symbol == "caller" && e.to_symbol == "xtotal"));
}

// ---- item 6: function declarations with modifiers / generics ----

#[test]
fn call_extraction_recognises_fn_modifiers_and_generics() {
    let source = "pub(crate) fn alpha() {\n    total(1);\n}\n\
                  async fn beta() {\n    other_call(2);\n}\n\
                  const fn gamma<T>(v: T) -> T {\n    helper(v)\n}\n\
                  unsafe fn delta() {\n    run_all();\n}\n";
    let graph = HeuristicCallExtractor::extract("src/lib.rs", source);

    let from = |to: &str| -> Vec<String> {
        graph.edges.iter().filter(|e| e.to_symbol == to).map(|e| e.from_symbol.clone()).collect()
    };
    assert_eq!(from("total"), vec!["alpha".to_string()]);
    assert_eq!(from("other_call"), vec!["beta".to_string()]);
    assert_eq!(from("helper"), vec!["gamma".to_string()]);
    assert_eq!(from("run_all"), vec!["delta".to_string()]);
    assert!(!graph.edges.iter().any(|e| e.from_symbol.contains('<')));
}

// ---- item 7: changed_path normalisation ----

#[test]
fn diff_impact_accepts_dot_relative_and_absolute_paths() {
    let dir = tempdir().unwrap();
    write(dir.path(), "src/lib.rs", "pub fn alpha() {}");
    let root = dir.path().to_string_lossy().to_string();

    for changed in [
        "src/lib.rs".to_string(),
        "./src/lib.rs".to_string(),
        format!("{root}/src/lib.rs"),
    ] {
        let impact = SemanticTools::diff_impact(DiffImpactRequest {
            repo_root: root.clone(),
            changed_path: changed.clone(),
        })
        .unwrap();
        assert!(
            impact.affected_symbols.iter().any(|s| s == "alpha"),
            "changed_path {changed} gave {:?}",
            impact.affected_symbols
        );
    }
}

// ---- item 8: import and test extraction ----

#[test]
fn import_extraction_recognises_restricted_visibility() {
    let source = "pub(crate) use crate::foo::Bar;\npub(super) use super::baz::Qux;\nuse std::fs;\n";
    let graph = RustImportExtractor::extract("src/lib.rs", source);
    let imports: Vec<String> = graph.edges.iter().map(|e| e.import.clone()).collect();
    assert_eq!(imports.len(), 3, "{imports:?}");
    assert!(imports.iter().any(|i| i.contains("crate::foo::Bar")));
    assert!(imports.iter().any(|i| i.contains("super::baz::Qux")));
}

#[test]
fn test_detection_requires_test_attribute_and_stores_name() {
    let source = "#[test]\nfn test_alpha() { alpha(); }\n\
                  fn test_helper() {}\n\
                  #[tokio::test]\nasync fn test_async() {}\n";
    let graph = RustTestExtractor::extract("tests/a.rs", source);
    let names: Vec<String> = graph.references.iter().map(|r| r.referenced_symbol.clone()).collect();
    assert_eq!(names, vec!["test_alpha".to_string(), "test_async".to_string()]);
}

// ---- item 2: semantic context relevance ----

#[test]
fn semantic_context_only_includes_goal_relevant_items() {
    let mut index = SemanticIndex::default();
    index.symbols.add(CodeSymbol {
        name: "build_context_package".into(),
        kind: SymbolKind::Function,
        path: "src/context.rs".into(),
        line_start: 1,
        line_end: 3,
        signature: None,
    });
    index.imports.add("src/net.rs", "use std::net::TcpStream");
    index.calls.add("main", "spawn", "src/main.rs");

    let items = SemanticContextAssembler::assemble(
        "context package",
        &index,
        SemanticContextOptions::default(),
    );

    assert!(!items.is_empty());
    assert!(items.iter().all(|item| !item.content.contains("TcpStream")), "{items:?}");
    assert!(items.iter().all(|item| !item.content.contains("spawn")), "{items:?}");
    assert!(items.iter().any(|item| matches!(item.kind, ContextItemKind::FileExcerpt)));
}

#[test]
fn semantic_context_is_empty_for_unrelated_goal() {
    let mut index = SemanticIndex::default();
    index.symbols.add(CodeSymbol {
        name: "build_context".into(),
        kind: SymbolKind::Function,
        path: "src/context.rs".into(),
        line_start: 1,
        line_end: 3,
        signature: None,
    });
    index.imports.add("src/net.rs", "use std::net::TcpStream");

    let items = SemanticContextAssembler::assemble("zebra", &index, SemanticContextOptions::default());
    assert!(items.is_empty(), "{items:?}");
}

// ---- item 3: size limits ----

#[test]
fn summary_caps_line_length() {
    let content = "x".repeat(1_000_000);
    let summary = SummaryGenerator::summarize_file("bundle.min.js", &content);
    assert!(summary.summary.len() < 4096, "summary len {}", summary.summary.len());
}

#[test]
fn task_context_skips_oversized_files() {
    let dir = tempdir().unwrap();
    write(dir.path(), "src/context_big.rs", &"a".repeat(2 * 1024 * 1024));

    let response = V4Tools::get_task_context(
        GetTaskContextRequest {
            task_id: None,
            goal: "context".to_string(),
            intent: TaskIntent::Analyze,
        },
        dir.path().to_string_lossy().to_string(),
    );
    assert!(
        response.context_package.budget.estimated_context_tokens < 10_000,
        "tokens {}",
        response.context_package.budget.estimated_context_tokens
    );
}

// ---- item 9: storage ----

#[test]
fn concurrent_jsonl_appends_stay_whole_lines() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_string_lossy().to_string();

    let handles: Vec<_> = (0..8)
        .map(|t| {
            let root = root.clone();
            std::thread::spawn(move || {
                let storage = V4Storage::new(&root);
                for i in 0..200 {
                    let line = format!("{{\"thread\":{t},\"i\":{i},\"pad\":\"{}\"}}", "p".repeat(300));
                    storage.append_jsonl("traces/concurrent.jsonl", &line).unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let content = fs::read_to_string(dir.path().join(".codeaware/v4/traces/concurrent.jsonl")).unwrap();
    let mut count = 0;
    for line in content.lines() {
        serde_json::from_str::<serde_json::Value>(line).expect("each line must be valid JSON");
        count += 1;
    }
    assert_eq!(count, 1600);
}
