use codeaware_mcp::v4::{CandidateDiscovery, DiscoveryConfig};
use tempfile::tempdir;

#[test]
fn discovery_finds_ranked_candidates() {
    let dir = tempdir().unwrap();

    std::fs::create_dir_all(dir.path().join("src/v4")).unwrap();

    std::fs::write(
        dir.path().join("src/v4/context.rs"),
        "pub struct ContextPackage;",
    )
    .unwrap();

    std::fs::write(
        dir.path().join("README.md"),
        "CodeAware",
    )
    .unwrap();

    let ranked = CandidateDiscovery::discover_ranked(
        dir.path(),
        "context package",
        DiscoveryConfig::default(),
    )
    .unwrap();

    assert!(!ranked.is_empty());
}
