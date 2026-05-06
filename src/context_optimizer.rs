#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextCompressionLevel {
    Aggressive,
    Medium,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolPolicy {
    MinimalTools,
    FocusTools,
    AllTools,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtendedThinkingPolicy {
    Auto,
    Off,
    On,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOptimizerPolicy {
    pub compression_level: ContextCompressionLevel,
    pub tool_policy: ToolPolicy,
    pub extended_thinking_policy: ExtendedThinkingPolicy,
}

impl Default for ContextOptimizerPolicy {
    fn default() -> Self {
        Self {
            compression_level: ContextCompressionLevel::Medium,
            tool_policy: ToolPolicy::FocusTools,
            extended_thinking_policy: ExtendedThinkingPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelevantCodeRequest {
    pub query: String,
    pub max_snippets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSearchResult {
    pub file_path: String,
    pub symbol: Option<String>,
    pub line: Option<u64>,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelevantCodeResponse {
    pub query: String,
    pub results: Vec<CodeSearchResult>,
    pub compression_note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestErrorSummary {
    pub summary: String,
    pub errors: Vec<String>,
    pub impacted_files: Vec<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContextSummary {
    pub role: String,
    pub architecture: Vec<String>,
    pub key_rules: Vec<String>,
    pub omitted_noise: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManagerDecision {
    pub enabled_tools: Vec<String>,
    pub disabled_tools: Vec<String>,
    pub extended_thinking: bool,
    pub reason: String,
}

pub fn get_relevant_code(query: &str, source: &str, max_snippets: usize) -> RelevantCodeResponse {
    let needle = query.to_lowercase();
    let mut results = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        if line.to_lowercase().contains(&needle) {
            results.push(CodeSearchResult {
                file_path: "inline_source".to_string(),
                symbol: extract_symbol_hint(line),
                line: Some((idx + 1) as u64),
                snippet: line.trim().to_string(),
            });
        }

        if results.len() >= max_snippets {
            break;
        }
    }

    RelevantCodeResponse {
        query: query.to_string(),
        results,
        compression_note: "Returned matching snippets instead of full source".to_string(),
    }
}

pub fn code_search(query: &str, files: &[(&str, &str)], max_results: usize) -> Vec<CodeSearchResult> {
    let needle = query.to_lowercase();
    let mut results = Vec::new();

    for (path, content) in files {
        for (idx, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                results.push(CodeSearchResult {
                    file_path: (*path).to_string(),
                    symbol: extract_symbol_hint(line),
                    line: Some((idx + 1) as u64),
                    snippet: line.trim().to_string(),
                });
            }

            if results.len() >= max_results {
                return results;
            }
        }
    }

    results
}

pub fn get_relevant_test_errors(output: &str, duration_ms: Option<u64>) -> TestErrorSummary {
    let mut errors = Vec::new();
    let mut impacted_files = Vec::new();

    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
            errors.push(line.trim().to_string());
        }
        if let Some(file) = extract_file_hint(line) {
            if !impacted_files.contains(&file) {
                impacted_files.push(file);
            }
        }
    }

    let summary = if errors.is_empty() {
        "No relevant test errors found".to_string()
    } else {
        format!("{} relevant error lines extracted", errors.len())
    };

    TestErrorSummary {
        summary,
        errors,
        impacted_files,
        duration_ms,
    }
}

pub fn reduce_project_context(input: &str) -> ProjectContextSummary {
    let mut architecture = Vec::new();
    let mut key_rules = Vec::new();
    let mut omitted_noise = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if trimmed.is_empty() {
            continue;
        }
        if lower.contains("architecture") || lower.contains("runtime") || lower.contains("database") {
            architecture.push(trimmed.to_string());
        } else if lower.contains("must") || lower.contains("never") || lower.contains("rule") {
            key_rules.push(trimmed.to_string());
        } else {
            omitted_noise.push(trimmed.to_string());
        }
    }

    ProjectContextSummary {
        role: "Concise project context for AI coding agents".to_string(),
        architecture,
        key_rules,
        omitted_noise,
    }
}

pub fn tool_manager(policy: ContextOptimizerPolicy, requested_tools: &[String]) -> ToolManagerDecision {
    let core_tools = [
        "smart_read".to_string(),
        "smart_run".to_string(),
        "token_stats".to_string(),
        "code_search".to_string(),
    ];

    let enabled_tools = match policy.tool_policy {
        ToolPolicy::MinimalTools => requested_tools
            .iter()
            .filter(|tool| core_tools.contains(tool))
            .cloned()
            .collect::<Vec<_>>(),
        ToolPolicy::FocusTools => requested_tools.iter().take(8).cloned().collect::<Vec<_>>(),
        ToolPolicy::AllTools => requested_tools.to_vec(),
    };

    let disabled_tools = requested_tools
        .iter()
        .filter(|tool| !enabled_tools.contains(tool))
        .cloned()
        .collect::<Vec<_>>();

    let extended_thinking = match policy.extended_thinking_policy {
        ExtendedThinkingPolicy::On => true,
        ExtendedThinkingPolicy::Off => false,
        ExtendedThinkingPolicy::Auto => !matches!(policy.compression_level, ContextCompressionLevel::Aggressive),
    };

    ToolManagerDecision {
        enabled_tools,
        disabled_tools,
        extended_thinking,
        reason: "Selected tools based on context optimization policy".to_string(),
    }
}

fn extract_symbol_hint(line: &str) -> Option<String> {
    let trimmed = line.trim();
    for prefix in ["fn ", "struct ", "enum ", "class ", "interface ", "trait "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .filter(|name| !name.is_empty())
                .map(|name| name.to_string());
        }
    }
    None
}

fn extract_file_hint(line: &str) -> Option<String> {
    for token in line.split_whitespace() {
        let cleaned = token.trim_matches(|c: char| c == ':' || c == ',' || c == ')' || c == '(');
        if cleaned.contains('/') && (cleaned.ends_with(".rs") || cleaned.ends_with(".py") || cleaned.ends_with(".ts") || cleaned.ends_with(".js") || cleaned.ends_with(".php")) {
            return Some(cleaned.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_relevant_code_snippets() {
        let source = "fn login() {}\nfn logout() {}";
        let result = get_relevant_code("login", source, 5);
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].symbol.as_deref(), Some("login"));
    }

    #[test]
    fn extracts_test_errors() {
        let output = "test failed\nerror in src/auth.rs:10\nok";
        let result = get_relevant_test_errors(output, Some(100));
        assert_eq!(result.errors.len(), 2);
        assert!(result.impacted_files.contains(&"src/auth.rs".to_string()));
    }

    #[test]
    fn reduces_project_context() {
        let input = "Architecture: Rust MCP runtime\nYou must keep responses deterministic\nLong example text";
        let summary = reduce_project_context(input);
        assert_eq!(summary.architecture.len(), 1);
        assert_eq!(summary.key_rules.len(), 1);
    }

    #[test]
    fn tool_manager_disables_non_core_tools() {
        let tools = vec!["smart_read".to_string(), "web_search".to_string()];
        let decision = tool_manager(
            ContextOptimizerPolicy {
                compression_level: ContextCompressionLevel::Aggressive,
                tool_policy: ToolPolicy::MinimalTools,
                extended_thinking_policy: ExtendedThinkingPolicy::Auto,
            },
            &tools,
        );
        assert!(decision.enabled_tools.contains(&"smart_read".to_string()));
        assert!(decision.disabled_tools.contains(&"web_search".to_string()));
        assert!(!decision.extended_thinking);
    }
}
