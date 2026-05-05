use crate::token_stats::{calculate_savings_ratio, estimate_tokens};

#[derive(Debug, Clone, PartialEq)]
pub struct TokenBenchmarkFixture {
    pub name: String,
    pub category: String,
    pub tool: String,
    pub subject: String,
    pub raw: String,
    pub compressed: String,
    pub expected_min_savings_ratio: f64,
    pub expected_max_savings_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenBenchmarkResult {
    pub name: String,
    pub category: String,
    pub tool: String,
    pub subject: String,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub expected_min_savings_ratio: f64,
    pub expected_max_savings_ratio: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenBenchmarkSummary {
    pub fixtures: u64,
    pub passed: u64,
    pub failed: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub average_savings_ratio: f64,
    pub results: Vec<TokenBenchmarkResult>,
}

pub fn run_token_benchmarks(fixtures: &[TokenBenchmarkFixture]) -> TokenBenchmarkSummary {
    let results: Vec<TokenBenchmarkResult> = fixtures.iter().map(run_fixture).collect();
    summarize_results(results)
}

pub fn run_fixture(fixture: &TokenBenchmarkFixture) -> TokenBenchmarkResult {
    let raw_tokens = estimate_tokens(&fixture.raw);
    let compressed_tokens = estimate_tokens(&fixture.compressed);
    let saved_tokens = raw_tokens as i64 - compressed_tokens as i64;
    let savings_ratio = calculate_savings_ratio(raw_tokens, compressed_tokens);
    let passed = savings_ratio >= fixture.expected_min_savings_ratio
        && savings_ratio <= fixture.expected_max_savings_ratio;

    TokenBenchmarkResult {
        name: fixture.name.clone(),
        category: fixture.category.clone(),
        tool: fixture.tool.clone(),
        subject: fixture.subject.clone(),
        raw_tokens,
        compressed_tokens,
        saved_tokens,
        savings_ratio,
        expected_min_savings_ratio: fixture.expected_min_savings_ratio,
        expected_max_savings_ratio: fixture.expected_max_savings_ratio,
        passed,
    }
}

pub fn summarize_results(results: Vec<TokenBenchmarkResult>) -> TokenBenchmarkSummary {
    let fixtures = results.len() as u64;
    let passed = results.iter().filter(|result| result.passed).count() as u64;
    let failed = fixtures.saturating_sub(passed);
    let raw_tokens = results.iter().map(|result| result.raw_tokens).sum();
    let compressed_tokens = results.iter().map(|result| result.compressed_tokens).sum();
    let saved_tokens = raw_tokens as i64 - compressed_tokens as i64;
    let average_savings_ratio = if fixtures == 0 {
        0.0
    } else {
        let total: f64 = results.iter().map(|result| result.savings_ratio).sum();
        let ratio = total / fixtures as f64;
        if ratio.is_finite() { ratio } else { 0.0 }
    };

    TokenBenchmarkSummary {
        fixtures,
        passed,
        failed,
        raw_tokens,
        compressed_tokens,
        saved_tokens,
        average_savings_ratio,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TokenBenchmarkFixture {
        TokenBenchmarkFixture {
            name: "sample".to_string(),
            category: "file_read".to_string(),
            tool: "smart_read".to_string(),
            subject: "src/server.rs".to_string(),
            raw: "fn main() { println!(\"hello\"); }\nfn health() -> &'static str { \"ok\" }".to_string(),
            compressed: "symbols:\n- main\n- health".to_string(),
            expected_min_savings_ratio: 0.1,
            expected_max_savings_ratio: 0.99,
        }
    }

    #[test]
    fn runs_single_fixture() {
        let result = run_fixture(&fixture());

        assert_eq!(result.name, "sample");
        assert_eq!(result.category, "file_read");
        assert_eq!(result.tool, "smart_read");
        assert!(result.raw_tokens > 0);
        assert!(result.compressed_tokens > 0);
        assert!(result.savings_ratio.is_finite());
    }

    #[test]
    fn marks_fixture_as_passed_when_ratio_is_in_range() {
        let result = run_fixture(&fixture());

        assert!(result.passed);
    }

    #[test]
    fn marks_fixture_as_failed_when_ratio_is_out_of_range() {
        let mut fixture = fixture();
        fixture.expected_min_savings_ratio = 0.99;
        fixture.expected_max_savings_ratio = 1.0;

        let result = run_fixture(&fixture);

        assert!(!result.passed);
    }

    #[test]
    fn summarizes_empty_results_safely() {
        let summary = summarize_results(Vec::new());

        assert_eq!(summary.fixtures, 0);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.average_savings_ratio, 0.0);
        assert!(summary.average_savings_ratio.is_finite());
    }

    #[test]
    fn summarizes_multiple_fixtures() {
        let fixtures = vec![fixture(), fixture()];
        let summary = run_token_benchmarks(&fixtures);

        assert_eq!(summary.fixtures, 2);
        assert_eq!(summary.results.len(), 2);
        assert!(summary.average_savings_ratio.is_finite());
    }
}
