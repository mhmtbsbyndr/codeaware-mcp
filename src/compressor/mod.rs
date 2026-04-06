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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_test_runners() {
        assert_eq!(classify_command("cargo test"), "test_runner");
        assert_eq!(classify_command("cargo test --release"), "test_runner");
        assert_eq!(classify_command("go test ./..."), "test_runner");
        assert_eq!(classify_command("npm test"), "test_runner");
        assert_eq!(classify_command("npx vitest run"), "test_runner");
        assert_eq!(classify_command("npx jest"), "test_runner");
        assert_eq!(classify_command("pytest -v"), "test_runner");
        assert_eq!(classify_command("jest --watch"), "test_runner");
        assert_eq!(classify_command("phpunit tests/"), "test_runner");
    }

    #[test]
    fn test_classify_compilers() {
        assert_eq!(classify_command("cargo build"), "compiler");
        assert_eq!(classify_command("cargo build --release"), "compiler");
        assert_eq!(classify_command("go build ."), "compiler");
        assert_eq!(classify_command("rustc main.rs"), "compiler");
        assert_eq!(classify_command("tsc --build"), "compiler");
        assert_eq!(classify_command("gcc -o app main.c"), "compiler");
        assert_eq!(classify_command("javac App.java"), "compiler");
    }

    #[test]
    fn test_classify_linters() {
        assert_eq!(classify_command("cargo clippy"), "linter");
        assert_eq!(classify_command("eslint src/"), "linter");
        assert_eq!(classify_command("ruff check ."), "linter");
        assert_eq!(classify_command("pylint module.py"), "linter");
    }

    #[test]
    fn test_classify_git() {
        assert_eq!(classify_command("git status"), "git_info");
        assert_eq!(classify_command("git diff HEAD"), "git_info");
        assert_eq!(classify_command("git log --oneline"), "git_info");
    }

    #[test]
    fn test_classify_package_managers() {
        assert_eq!(classify_command("cargo add serde"), "package_mgr");
        assert_eq!(classify_command("npm install express"), "package_mgr");
        assert_eq!(classify_command("pip install requests"), "package_mgr");
        assert_eq!(classify_command("go get golang.org/x/sync"), "package_mgr");
    }

    #[test]
    fn test_classify_formatters() {
        assert_eq!(classify_command("rustfmt src/main.rs"), "formatter");
        assert_eq!(classify_command("prettier --write ."), "formatter");
        assert_eq!(classify_command("black ."), "formatter");
        assert_eq!(classify_command("gofmt -w ."), "formatter");
    }

    #[test]
    fn test_classify_search() {
        assert_eq!(classify_command("grep -r pattern ."), "search");
        assert_eq!(classify_command("rg pattern"), "search");
        assert_eq!(classify_command("find . -name '*.rs'"), "search");
    }

    #[test]
    fn test_classify_generic_fallback() {
        assert_eq!(classify_command("echo hello"), "generic");
        assert_eq!(classify_command("ls -la"), "generic");
        assert_eq!(classify_command("cat file.txt"), "generic");
    }

    #[test]
    fn test_classify_whitespace_handling() {
        assert_eq!(classify_command("  cargo test  "), "test_runner");
        assert_eq!(classify_command(""), "generic");
    }

    #[test]
    fn test_compress_output_dispatches_correctly() {
        // Verify dispatch doesn't panic for each category
        let output = "some test output";
        for category in &["test_runner", "compiler", "linter", "search", "git_info", "package_mgr", "formatter", "generic"] {
            let _ = compress_output(category, output, 50);
        }
    }
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
