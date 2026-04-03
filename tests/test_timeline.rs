use codeaware_mcp::xray::metrics::MetricsState;

#[test]
fn test_timeline_event_recorded() {
    let mut m = MetricsState::new();
    m.record_timeline_event("smart_read", Some("src/main.rs"), 500, 50, 12, "Analyzing");
    let snap = m.snapshot();
    assert_eq!(snap.timeline.len(), 1);
    assert_eq!(snap.timeline[0].tool, "smart_read");
    assert_eq!(snap.timeline[0].duration_ms, 12);
}

#[test]
fn test_timeline_in_snapshot() {
    let mut m = MetricsState::new();
    m.record_timeline_event("smart_run", None, 200, 20, 5, "Verifying");
    m.record_timeline_event("smart_edit", Some("src/lib.rs"), 100, 50, 8, "Editing");
    let snap = m.snapshot();
    assert_eq!(snap.timeline.len(), 2);
    assert_eq!(snap.timeline[0].tool, "smart_run");
    assert_eq!(snap.timeline[1].tool, "smart_edit");
}

#[test]
fn test_timeline_serializes() {
    let mut m = MetricsState::new();
    m.record_timeline_event("project_map", None, 300, 30, 15, "Analyzing");
    let snap = m.snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    assert!(json.contains("\"timeline\""));
    assert!(json.contains("\"duration_ms\":15"));
}

#[test]
fn test_multiple_events_ordered() {
    let mut m = MetricsState::new();
    m.record_timeline_event("smart_read", Some("a.rs"), 100, 10, 1, "Analyzing");
    m.record_timeline_event("smart_edit", Some("a.rs"), 200, 100, 2, "Editing");
    m.record_timeline_event("smart_run", None, 300, 30, 3, "Verifying");
    let snap = m.snapshot();
    assert_eq!(snap.timeline.len(), 3);
    assert_eq!(snap.timeline[0].tool, "smart_read");
    assert_eq!(snap.timeline[2].tool, "smart_run");
}
