use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedTest {
    pub test_file: String,
    pub test_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSelection {
    pub selected_tests: Vec<SelectedTest>,
    pub command: String,
    pub coverage_estimate: String,
}

pub fn select_tests(
    edited_file: &str,
    edited_symbols: &[String],
    project_root: &Path,
) -> TestSelection {
    let mut selected = Vec::new();

    // Derive module name from file path
    // e.g. "src/auth.rs" -> "auth", "src/tools/smart_edit.rs" -> "smart_edit"
    let module_name = Path::new(edited_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // Scan tests/ directory for test files
    let tests_dir = project_root.join("tests");
    if tests_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&tests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let fname = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Match test file conventions
                if !fname.ends_with(".rs")
                    && !fname.ends_with(".py")
                    && !fname.ends_with(".test.ts")
                    && !fname.ends_with(".test.js")
                {
                    continue;
                }

                // Read test file content
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // Check if test file references the edited module or symbols
                let mut reasons = Vec::new();

                // Check module import
                if !module_name.is_empty() && content.contains(module_name) {
                    reasons.push(format!("references module '{}'", module_name));
                }

                // Check symbol references
                for sym in edited_symbols {
                    if content.contains(sym.as_str()) {
                        reasons.push(format!("references symbol '{}'", sym));
                    }
                }

                if !reasons.is_empty() {
                    let rel_path = path
                        .strip_prefix(project_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    selected.push(SelectedTest {
                        test_file: rel_path,
                        test_name: None,
                        reason: reasons.join(", "),
                    });
                }
            }
        }
    }

    // Build command
    let command = if selected.is_empty() {
        // Fallback to full suite
        detect_test_command(project_root)
    } else {
        build_selective_command(&selected, project_root)
    };

    let total_test_files = count_test_files(project_root);
    let coverage_estimate = if selected.is_empty() {
        format!("full suite ({} test files)", total_test_files)
    } else {
        format!(
            "{} of {} test files (covers edited symbols)",
            selected.len(),
            total_test_files
        )
    };

    TestSelection {
        selected_tests: selected,
        command,
        coverage_estimate,
    }
}

fn detect_test_command(project_root: &Path) -> String {
    if project_root.join("Cargo.toml").exists() {
        "cargo test".to_string()
    } else if project_root.join("package.json").exists() {
        "npm test".to_string()
    } else if project_root.join("pytest.ini").exists() || project_root.join("setup.py").exists() {
        "pytest".to_string()
    } else {
        "cargo test".to_string() // default
    }
}

fn build_selective_command(tests: &[SelectedTest], project_root: &Path) -> String {
    if project_root.join("Cargo.toml").exists() {
        // Rust: cargo test --test <test_file_stem> for each
        let test_args: Vec<String> = tests
            .iter()
            .filter_map(|t| {
                Path::new(&t.test_file)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| format!("--test {}", s))
            })
            .collect();
        format!("cargo test {}", test_args.join(" "))
    } else if project_root.join("package.json").exists() {
        let files: Vec<&str> = tests.iter().map(|t| t.test_file.as_str()).collect();
        format!("npx jest {}", files.join(" "))
    } else {
        let files: Vec<&str> = tests.iter().map(|t| t.test_file.as_str()).collect();
        format!("pytest {}", files.join(" "))
    }
}

fn count_test_files(project_root: &Path) -> usize {
    let tests_dir = project_root.join("tests");
    if !tests_dir.is_dir() {
        return 0;
    }
    std::fs::read_dir(&tests_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.ends_with(".rs")
                        || name.ends_with(".py")
                        || name.ends_with(".test.ts")
                        || name.ends_with(".test.js")
                })
                .count()
        })
        .unwrap_or(0)
}
