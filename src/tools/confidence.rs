use serde::Serialize;

/// Internal-only input for confidence computation — not serialized.
pub struct ConfidenceInput<'a> {
    pub test_file_exists: bool,
    pub symbol_in_test: bool,
    pub callers_affected: usize,
    pub trust_level: &'a str,       // "exact", "structural", "heuristic", "degraded", "raw"
    pub git_changes_last_10: i32,   // -1 = unavailable
    pub is_public: bool,
    pub signature_changed: bool,
    pub has_unsafe: bool,
    pub error_type_widened: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorScore {
    pub score: u32,
    pub weight: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidenceFactors {
    pub test_coverage: FactorScore,
    pub caller_impact: FactorScore,
    pub type_safety: FactorScore,
    pub git_stability: FactorScore,
    pub semantic_risk: FactorScore,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfidenceScore {
    pub score: u32,
    pub verdict: String,
    pub factors: ConfidenceFactors,
    pub weakest: String,
    pub suggestion: String,
}

pub fn compute_confidence(input: &ConfidenceInput) -> ConfidenceScore {
    // --- test_coverage (weight 0.30) ---
    let test_coverage_score = if input.test_file_exists && input.symbol_in_test {
        100
    } else if input.test_file_exists {
        50
    } else {
        0
    };
    let test_coverage = FactorScore {
        score: test_coverage_score,
        weight: 0.30,
        detail: match test_coverage_score {
            100 => "test file exists and symbol is tested".into(),
            50 => "test file exists but symbol not directly tested".into(),
            _ => "no test file found".into(),
        },
    };

    // --- caller_impact (weight 0.20) ---
    let caller_impact_score = match input.callers_affected {
        0 => 100,
        1 => 90,
        2..=3 => 70,
        4..=10 => 50,
        _ => 20,
    };
    let caller_impact = FactorScore {
        score: caller_impact_score,
        weight: 0.20,
        detail: format!("{} callers affected", input.callers_affected),
    };

    // --- type_safety (weight 0.20) ---
    let type_safety_score = match input.trust_level {
        "exact" => 100,
        "structural" => 80,
        "heuristic" => 50,
        "degraded" => 30,
        _ => 20,
    };
    let type_safety = FactorScore {
        score: type_safety_score,
        weight: 0.20,
        detail: format!("trust level: {}", input.trust_level),
    };

    // --- git_stability (weight 0.15) ---
    let git_stability_score = match input.git_changes_last_10 {
        -1 => 60,
        0 => 100,
        1..=2 => 80,
        3..=5 => 60,
        6..=10 => 40,
        _ => 20,
    };
    let git_stability = FactorScore {
        score: git_stability_score,
        weight: 0.15,
        detail: if input.git_changes_last_10 == -1 {
            "git data unavailable, using neutral score".into()
        } else {
            format!("{} changes in last 10 commits", input.git_changes_last_10)
        },
    };

    // --- semantic_risk (weight 0.15) ---
    let mut semantic_risk_val: i32 = 100;
    if input.is_public {
        semantic_risk_val -= 20;
    }
    if input.signature_changed {
        semantic_risk_val -= 30;
    }
    if input.has_unsafe {
        semantic_risk_val -= 20;
    }
    if input.error_type_widened {
        semantic_risk_val -= 20;
    }
    let semantic_risk_score = semantic_risk_val.max(0) as u32;

    let mut risk_details = Vec::new();
    if input.is_public {
        risk_details.push("public API");
    }
    if input.signature_changed {
        risk_details.push("signature changed");
    }
    if input.has_unsafe {
        risk_details.push("contains unsafe");
    }
    if input.error_type_widened {
        risk_details.push("error type widened");
    }
    let semantic_risk = FactorScore {
        score: semantic_risk_score,
        weight: 0.15,
        detail: if risk_details.is_empty() {
            "no semantic risks detected".into()
        } else {
            risk_details.join(", ")
        },
    };

    // --- weighted sum ---
    let weighted_sum = (test_coverage.score as f64 * test_coverage.weight)
        + (caller_impact.score as f64 * caller_impact.weight)
        + (type_safety.score as f64 * type_safety.weight)
        + (git_stability.score as f64 * git_stability.weight)
        + (semantic_risk.score as f64 * semantic_risk.weight);

    let final_score = weighted_sum.round() as u32;

    let verdict = if final_score >= 80 {
        "safe".to_string()
    } else if final_score >= 60 {
        "review".to_string()
    } else {
        "risky".to_string()
    };

    // --- weakest factor ---
    let factors_list: [(&str, u32); 5] = [
        ("test_coverage", test_coverage.score),
        ("caller_impact", caller_impact.score),
        ("type_safety", type_safety.score),
        ("git_stability", git_stability.score),
        ("semantic_risk", semantic_risk.score),
    ];
    let weakest_entry = factors_list
        .iter()
        .min_by_key(|(_, s)| *s)
        .unwrap();
    let weakest = weakest_entry.0.to_string();

    let suggestion = match weakest.as_str() {
        "test_coverage" => "Add or update tests to cover the modified symbol.".to_string(),
        "caller_impact" => "Review all callers to ensure compatibility with the change.".to_string(),
        "type_safety" => "Consider using a language server for precise type analysis.".to_string(),
        "git_stability" => "This file changes frequently — consider refactoring to reduce churn.".to_string(),
        "semantic_risk" => "Public API or unsafe change — ensure backward compatibility.".to_string(),
        _ => "Review the change carefully.".to_string(),
    };

    let factors = ConfidenceFactors {
        test_coverage,
        caller_impact,
        type_safety,
        git_stability,
        semantic_risk,
    };

    ConfidenceScore {
        score: final_score,
        verdict,
        factors,
        weakest,
        suggestion,
    }
}
