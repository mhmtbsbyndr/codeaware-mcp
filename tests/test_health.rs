use codeaware_mcp::session::health::*;

#[test]
fn test_compute_health_perfect() {
    let factors = HealthFactors {
        test_coverage: 100,
        stability: 100,
        error_rate: 100,
        complexity: 100,
        documentation: 100,
    };
    assert_eq!(compute_health(&factors), 100);
}

#[test]
fn test_compute_health_weighted() {
    let factors = HealthFactors {
        test_coverage: 80, // * 0.25 = 20
        stability: 60,     // * 0.20 = 12
        error_rate: 40,    // * 0.20 = 8
        complexity: 70,    // * 0.20 = 14
        documentation: 50, // * 0.15 = 7.5
    };
    // 20 + 12 + 8 + 14 + 7.5 = 61.5 → 62
    assert_eq!(compute_health(&factors), 62);
}

#[test]
fn test_scores_after_read() {
    let (complexity, doc) = scores_after_read(150, 10, true);
    assert_eq!(complexity, 70); // 101-200 LOC
    assert_eq!(doc, 80); // has docs + symbols > 0
}

#[test]
fn test_stability_after_edit() {
    assert_eq!(stability_after_edit(80), 75);
    assert_eq!(stability_after_edit(10), 10); // min 10
    assert_eq!(stability_after_edit(3), 10); // clamped to 10
}

#[test]
fn test_scores_after_test_pass() {
    let (test, error) = scores_after_test(true, 50, 50);
    assert_eq!(test, 60); // +10
    assert_eq!(error, 55); // +5
}

#[test]
fn test_scores_after_test_fail() {
    let (test, error) = scores_after_test(false, 50, 50);
    assert_eq!(test, 50); // unchanged
    assert_eq!(error, 35); // -15
}
