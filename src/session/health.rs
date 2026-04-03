use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HealthFactors {
    pub test_coverage: u32,
    pub stability: u32,
    pub error_rate: u32,
    pub complexity: u32,
    pub documentation: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeHealth {
    pub file_path: String,
    pub health_score: u32,
    pub factors: HealthFactors,
    pub trend: String,
    pub last_updated: String,
}

pub fn compute_health(factors: &HealthFactors) -> u32 {
    let weighted = (factors.test_coverage as f64 * 0.25)
        + (factors.stability as f64 * 0.20)
        + (factors.error_rate as f64 * 0.20)
        + (factors.complexity as f64 * 0.20)
        + (factors.documentation as f64 * 0.15);
    weighted.round() as u32
}

/// Update complexity and documentation scores after a smart_read
pub fn scores_after_read(loc: usize, symbol_count: usize, has_doc_comments: bool) -> (u32, u32) {
    // Complexity: lower LOC and fewer symbols = healthier
    let complexity = match loc {
        0..=50 => 100,
        51..=100 => 85,
        101..=200 => 70,
        201..=500 => 50,
        _ => 30,
    };

    // Documentation: has doc comments = healthier
    let doc = if has_doc_comments {
        if symbol_count > 0 { 80 } else { 60 }
    } else if symbol_count > 5 {
        20
    } else {
        40
    };

    (complexity, doc)
}

/// Update stability score after an edit (each edit slightly decreases stability)
pub fn stability_after_edit(current_stability: u32) -> u32 {
    // Each edit reduces stability by 5, min 10
    current_stability.saturating_sub(5).max(10)
}

/// Update test/error scores after a test run
pub fn scores_after_test(test_passed: bool, current_test: u32, current_error: u32) -> (u32, u32) {
    if test_passed {
        // Test passing improves both scores
        let test = (current_test + 10).min(100);
        let error = (current_error + 5).min(100);
        (test, error)
    } else {
        // Test failing decreases error score
        let test = current_test; // unchanged — test exists, just failing
        let error = current_error.saturating_sub(15);
        (test, error)
    }
}
