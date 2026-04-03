pub mod compiler_output;
pub mod formatter_output;
pub mod generic;
pub mod git_output;
pub mod linter_output;
pub mod package_mgr_output;
pub mod search_output;
pub mod test_output;

/// Classify a shell command string into a category.
pub fn classify_command(command: &str) -> &'static str {
    let cmd = command.trim();

    // Helpers: first word and two-word prefix
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    let two_words: String = {
        let mut words = cmd.split_whitespace();
        match (words.next(), words.next()) {
            (Some(a), Some(b)) => format!("{} {}", a, b),
            (Some(a), None) => a.to_string(),
            _ => String::new(),
        }
    };
    let tw = two_words.as_str();

    // test_runner (checked before compiler so "cargo test" wins over "cargo build")
    // Two-word prefixes
    if matches!(tw, "cargo test" | "go test" | "npm test" | "npx vitest" | "npx jest") {
        return "test_runner";
    }
    // Single-word tools (first_word match, covers "pytest -v", "jest --watch", etc.)
    if matches!(first_word, "pytest" | "jest" | "phpunit") {
        return "test_runner";
    }

    // compiler
    if matches!(tw, "cargo build" | "go build") {
        return "compiler";
    }
    if matches!(first_word, "rustc" | "tsc" | "gcc" | "g++" | "javac") {
        return "compiler";
    }

    // linter
    if matches!(tw, "cargo clippy") {
        return "linter";
    }
    if matches!(first_word, "eslint" | "ruff" | "phpstan" | "golangci-lint" | "pylint") {
        return "linter";
    }

    // git_info
    if cmd.starts_with("git ") {
        return "git_info";
    }

    // package_mgr
    if matches!(
        tw,
        "cargo add" | "npm install" | "pip install" | "go get" | "yarn add"
    ) {
        return "package_mgr";
    }

    // formatter (match first word)
    if matches!(first_word, "rustfmt" | "prettier" | "black" | "gofmt") {
        return "formatter";
    }

    // search (match first word)
    if matches!(first_word, "grep" | "rg" | "ag" | "find" | "fd") {
        return "search";
    }

    "generic"
}

/// Dispatch to type-specific compressor.
pub fn compress_output(command_type: &str, raw_output: &str, max_lines: usize) -> String {
    match command_type {
        "test_runner" => test_output::compress(raw_output, max_lines),
        "compiler" => compiler_output::compress(raw_output, max_lines),
        "linter" => linter_output::compress(raw_output, max_lines),
        "search" => search_output::compress(raw_output, max_lines),
        "git_info" => git_output::compress(raw_output, max_lines),
        "package_mgr" => package_mgr_output::compress(raw_output, max_lines),
        "formatter" => formatter_output::compress(raw_output, max_lines),
        _ => generic::compress(raw_output, max_lines),
    }
}
