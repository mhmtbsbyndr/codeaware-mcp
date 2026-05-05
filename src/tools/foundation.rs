use serde_json::{json, Value};

use crate::feedback_layer::{feedback_prompt, validate_feedback_rating};
use crate::token_benchmark::{run_token_benchmarks, TokenBenchmarkFixture};
use crate::token_quality::{evaluate_quality, TestRunSummary};
use crate::token_stats::{summarize_events, TokenEvent, TokenEventCategory};
use crate::token_stats_tools::{
    render_token_savings_report, TokenSavingsReportFormat, TokenStatsGroupBy, TokenStatsResponse,
};

pub fn handle_token_stats(_input: &Value) -> Value {
    let events = sample_events();
    let summary = summarize_events(&events);
    let response = TokenStatsResponse::from_summary(summary, TokenStatsGroupBy::Tool);

    json!({
        "ok": true,
        "trust": "exact",
        "data": {
            "events": response.events,
            "raw_tokens": response.raw_tokens,
            "compressed_tokens": response.compressed_tokens,
            "saved_tokens": response.saved_tokens,
            "savings_ratio": response.savings_ratio,
            "group_by": response.group_by,
            "buckets": response.buckets.iter().map(|bucket| json!({
                "name": bucket.name,
                "events": bucket.events,
                "raw_tokens": bucket.raw_tokens,
                "compressed_tokens": bucket.compressed_tokens,
                "saved_tokens": bucket.saved_tokens,
                "savings_ratio": bucket.savings_ratio,
            })).collect::<Vec<_>>()
        }
    })
}

pub fn handle_token_savings_report(_input: &Value) -> Value {
    let events = sample_events();
    let summary = summarize_events(&events);
    let report = render_token_savings_report(&summary, TokenSavingsReportFormat::Markdown);

    json!({
        "ok": true,
        "trust": "exact",
        "data": {
            "markdown": report.content
        }
    })
}

pub fn handle_benchmark_compression(_input: &Value) -> Value {
    let fixtures = vec![TokenBenchmarkFixture {
        name: "sample_rust_module".to_string(),
        category: "file_read".to_string(),
        tool: "smart_read".to_string(),
        subject: "src/server.rs".to_string(),
        raw: "fn main() { println!(\"hello\"); }\nfn health() -> &'static str { \"ok\" }".to_string(),
        compressed: "symbols:\n- main\n- health".to_string(),
        expected_min_savings_ratio: 0.1,
        expected_max_savings_ratio: 0.99,
    }];
    let summary = run_token_benchmarks(&fixtures);

    json!({
        "ok": true,
        "trust": "exact",
        "data": {
            "fixtures": summary.fixtures,
            "passed": summary.passed,
            "failed": summary.failed,
            "raw_tokens": summary.raw_tokens,
            "compressed_tokens": summary.compressed_tokens,
            "saved_tokens": summary.saved_tokens,
            "average_savings_ratio": summary.average_savings_ratio,
            "results": summary.results.iter().map(|result| json!({
                "name": result.name,
                "category": result.category,
                "tool": result.tool,
                "subject": result.subject,
                "raw_tokens": result.raw_tokens,
                "compressed_tokens": result.compressed_tokens,
                "saved_tokens": result.saved_tokens,
                "savings_ratio": result.savings_ratio,
                "passed": result.passed,
            })).collect::<Vec<_>>()
        }
    })
}

pub fn handle_provide_feedback(input: &Value) -> Value {
    let rating = input.get("rating").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let valid = validate_feedback_rating(rating);

    json!({
        "ok": valid,
        "trust": "exact",
        "data": {
            "accepted": valid,
            "rating": rating,
            "prompt": feedback_prompt(),
            "comment": input.get("comment").and_then(|v| v.as_str()).unwrap_or("")
        }
    })
}

pub fn handle_token_quality(input: &Value) -> Value {
    let summary = TestRunSummary {
        command: input.get("command").and_then(|v| v.as_str()).unwrap_or("cargo test").to_string(),
        exit_code: input.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        passed: input.get("passed").and_then(|v| v.as_u64()).unwrap_or(0),
        failed: input.get("failed").and_then(|v| v.as_u64()).unwrap_or(0),
        duration_ms: input.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0),
    };
    let evaluation = evaluate_quality(&summary);

    json!({
        "ok": true,
        "trust": "exact",
        "data": {
            "rating": format!("{:?}", evaluation.rating),
            "summary": evaluation.summary
        }
    })
}

fn sample_events() -> Vec<TokenEvent> {
    vec![TokenEvent::new(
        "event-1",
        "trace-1",
        "current",
        "smart_read",
        TokenEventCategory::FileRead,
        "src/server.rs",
        "fn main() { println!(\"hello\"); }\nfn health() -> &'static str { \"ok\" }",
        "symbols:\n- main\n- health",
        "2026-05-06T00:00:00Z",
    )]
}
