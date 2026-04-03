pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    let total = lines.len();
    let kept = &lines[..max_lines];
    let remaining = total - max_lines;

    let mut result: Vec<String> = kept.iter().map(|s| s.to_string()).collect();
    if remaining > 0 {
        result.push(format!("... {} more results", remaining));
    }

    result.join("\n")
}
