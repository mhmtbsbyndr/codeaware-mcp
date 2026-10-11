use codeaware_mcp::v4::context::{content_hash, delta_for, ServedMap, ServedVersion};
use codeaware_mcp::v4::{estimate_tokens, Representation};

fn long_file(changed_line: usize, replacement: &str) -> String {
    (1..=300)
        .map(|i| {
            if i == changed_line {
                replacement.to_string()
            } else {
                format!("let value_{i} = compute_{i}(input, {i});")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn first_read_is_full() {
    let now = "fn main() {}\n";
    assert_eq!(delta_for(None, now), Representation::Full(now.to_string()));
}

#[test]
fn unchanged_file_returns_one_line_marker() {
    let content = long_file(0, "");
    let prev = ServedVersion::new(&content);
    let rep = delta_for(Some(&prev), &content);
    let expected = format!("unchanged since {}", &content_hash(&content)[..8]);
    assert_eq!(rep, Representation::Unchanged(expected.clone()));
    assert_eq!(rep.text(), expected);
    assert!(estimate_tokens(rep.text()) < estimate_tokens(&content) / 10);
}

#[test]
fn small_change_in_large_file_returns_smaller_diff() {
    let before = long_file(0, "");
    let after = long_file(150, "let value_150 = compute_150(input, 999);");
    let prev = ServedVersion::new(&before);
    match delta_for(Some(&prev), &after) {
        Representation::Diff(diff) => {
            assert!(diff.contains("@@"), "{diff}");
            assert!(diff.contains("-let value_150 = compute_150(input, 150);"));
            assert!(diff.contains("+let value_150 = compute_150(input, 999);"));
            assert!(estimate_tokens(&diff) < estimate_tokens(&after));
        }
        other => panic!("expected diff, got {other:?}"),
    }
}

#[test]
fn change_in_tiny_file_falls_back_to_full_body() {
    let prev = ServedVersion::new("a\n");
    let rep = delta_for(Some(&prev), "b\n");
    assert_eq!(rep, Representation::Full("b\n".to_string()));
}

#[test]
fn served_map_tracks_last_served_version_per_path() {
    let mut served = ServedMap::default();
    let v1 = long_file(0, "");
    let v2 = long_file(10, "let value_10 = 0;");

    assert!(matches!(served.serve("src/a.rs", &v1), Representation::Full(_)));
    assert!(matches!(served.serve("src/a.rs", &v1), Representation::Unchanged(_)));
    assert!(matches!(served.serve("src/a.rs", &v2), Representation::Diff(_)));
    assert!(matches!(served.serve("src/b.rs", &v1), Representation::Full(_)), "paths are independent");

    assert_eq!(served.len(), 2);
    assert_eq!(served.get("src/a.rs").map(|v| v.content.as_str()), Some(v2.as_str()));
    assert_eq!(served.get("src/a.rs").map(|v| v.hash.clone()), Some(content_hash(&v2)));
}

#[test]
fn content_hash_is_stable_full_blake3_hex() {
    let hash = content_hash("hello");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(hash, content_hash("hello"));
    assert_ne!(hash, content_hash("hello!"));
}
