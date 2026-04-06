use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::intelligence::tree_sitter_provider::{SymbolKind, TreeSitterProvider};
use crate::tools::smart_read::detect_language;

use ignore::WalkBuilder;

/// Maximum file size to parse (100 KB).
const MAX_FILE_SIZE: u64 = 100 * 1024;

#[derive(Debug, Serialize)]
pub struct CoverageMap {
    pub total_functions: usize,
    pub tested_functions: usize,
    pub coverage_percent: f64,
    pub files: Vec<FileCoverage>,
    pub untested: Vec<UntestedFunction>,
}

#[derive(Debug, Serialize)]
pub struct FileCoverage {
    pub path: String,
    pub functions: usize,
    pub tested: usize,
    pub test_files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UntestedFunction {
    pub file: String,
    pub name: String,
    pub line: usize,
}

/// Entry point called from server dispatch.
pub fn handle_test_coverage_map(params: &Value) -> Value {
    let root = params
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let language_filter = params.get("language").and_then(|v| v.as_str());

    let root_path = PathBuf::from(root);
    if !root_path.is_dir() {
        let envelope = Envelope::<()>::error(
            ErrorCode::EInternalError,
            false,
            Some(format!("Path is not a directory: {root}")),
        );
        return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
    }

    match build_coverage_map(&root_path, language_filter) {
        Ok(map) => {
            let envelope = Envelope::success(&map, TrustLevel::Heuristic);
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
        Err(e) => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some(e),
            );
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
    }
}

/// Collected info about a single source file.
struct SourceFile {
    rel_path: String,
    functions: Vec<(String, usize)>, // (name, line)
}

/// Build the full coverage map for a project root.
fn build_coverage_map(
    root: &Path,
    language_filter: Option<&str>,
) -> Result<CoverageMap, String> {
    let provider = TreeSitterProvider::new();

    let mut source_files: Vec<SourceFile> = Vec::new();
    let mut test_files: Vec<(String, String)> = Vec::new(); // (rel_path, content)

    // Walk project tree respecting .gitignore
    let walker = WalkBuilder::new(root)
        .hidden(true) // skip hidden files
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Skip files exceeding size limit
        if let Ok(meta) = path.metadata() {
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }
        }

        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Detect language from extension
        let lang = match detect_language(&rel) {
            Some(l) => l,
            None => continue,
        };

        // Apply optional language filter
        if let Some(filter) = language_filter {
            if lang != filter {
                continue;
            }
        }

        let is_test = is_test_file(&rel, lang, path);

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if is_test {
            test_files.push((rel, content));
        } else {
            // Extract function/method symbols
            let symbols = match provider.extract_symbols(&content, lang) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let functions: Vec<(String, usize)> = symbols
                .into_iter()
                .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
                .map(|s| (s.name, s.start_line))
                .collect();

            if !functions.is_empty() {
                source_files.push(SourceFile {
                    rel_path: rel,
                    functions,
                });
            }
        }
    }

    // Cross-reference: for each source function, check if any test file mentions it.
    let mut total_functions = 0usize;
    let mut tested_functions = 0usize;
    let mut file_coverages: Vec<FileCoverage> = Vec::new();
    let mut untested: Vec<UntestedFunction> = Vec::new();

    for src in &source_files {
        let mut file_tested = 0usize;
        let mut matching_test_files: Vec<String> = Vec::new();

        for (fn_name, fn_line) in &src.functions {
            total_functions += 1;
            let mut found = false;

            for (test_rel, test_content) in &test_files {
                if test_content.contains(fn_name.as_str()) {
                    found = true;
                    if !matching_test_files.contains(test_rel) {
                        matching_test_files.push(test_rel.clone());
                    }
                }
            }

            if found {
                file_tested += 1;
                tested_functions += 1;
            } else {
                untested.push(UntestedFunction {
                    file: src.rel_path.clone(),
                    name: fn_name.clone(),
                    line: *fn_line,
                });
            }
        }

        file_coverages.push(FileCoverage {
            path: src.rel_path.clone(),
            functions: src.functions.len(),
            tested: file_tested,
            test_files: matching_test_files,
        });
    }

    // Sort: least covered files first
    file_coverages.sort_by(|a, b| {
        let cov_a = if a.functions == 0 {
            1.0
        } else {
            a.tested as f64 / a.functions as f64
        };
        let cov_b = if b.functions == 0 {
            1.0
        } else {
            b.tested as f64 / b.functions as f64
        };
        cov_a
            .partial_cmp(&cov_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let coverage_percent = if total_functions == 0 {
        0.0
    } else {
        (tested_functions as f64 / total_functions as f64 * 100.0 * 10.0).round() / 10.0
    };

    Ok(CoverageMap {
        total_functions,
        tested_functions,
        coverage_percent,
        files: file_coverages,
        untested,
    })
}

/// Determine whether a file is a test file based on language conventions.
fn is_test_file(rel_path: &str, language: &str, abs_path: &Path) -> bool {
    let file_name = abs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Check common test directories
    let in_tests_dir = rel_path.starts_with("tests/")
        || rel_path.starts_with("tests\\")
        || rel_path.contains("/tests/")
        || rel_path.contains("\\tests\\")
        || rel_path.starts_with("test/")
        || rel_path.starts_with("test\\")
        || rel_path.contains("/test/")
        || rel_path.contains("\\test\\");

    match language {
        "rust" => {
            // Rust: files in tests/ dir, or files containing #[test] / #[cfg(test)]
            if in_tests_dir {
                return true;
            }
            // Check file content for test attributes (only for inline tests)
            if let Ok(content) = std::fs::read_to_string(abs_path) {
                if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                    return true;
                }
            }
            false
        }
        "python" => {
            // Python: test_*.py, *_test.py, or in tests/ dir
            file_name.starts_with("test_")
                || file_name.ends_with("_test.py")
                || in_tests_dir
        }
        "typescript" | "javascript" | "tsx" | "jsx" => {
            // JS/TS: *.test.ts, *.spec.ts, or in __tests__/
            file_name.contains(".test.")
                || file_name.contains(".spec.")
                || rel_path.contains("__tests__/")
                || rel_path.contains("__tests__\\")
                || in_tests_dir
        }
        "java" => {
            // Java: *Test.java, *Spec.java, or in test/ dir
            file_name.ends_with("Test.java")
                || file_name.ends_with("Spec.java")
                || in_tests_dir
        }
        "c" | "cpp" => {
            // C/C++: in test/ or tests/ dir
            in_tests_dir
        }
        _ => in_tests_dir,
    }
}
