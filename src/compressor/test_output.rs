struct FailureBlock {
    test_name: String,
    message: String,
}

fn extract_failure_blocks(raw: &str) -> Vec<FailureBlock> {
    let mut failures = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Detect cargo test failure block: "---- <test_name> stdout ----"
        if lines[i].starts_with("---- ") && lines[i].ends_with(" stdout ----") {
            let test_name = lines[i]
                .trim_start_matches("---- ")
                .trim_end_matches(" stdout ----")
                .to_string();
            let mut message_parts = Vec::new();
            i += 1;
            // Collect up to 3 meaningful lines for the message
            while i < lines.len() && !lines[i].starts_with("---- ") && !lines[i].starts_with("failures:") {
                let line = lines[i].trim();
                if !line.is_empty() && message_parts.len() < 3 {
                    message_parts.push(line.to_string());
                }
                i += 1;
            }
            let message = message_parts.join(" | ");
            failures.push(FailureBlock { test_name, message });
        } else {
            i += 1;
        }
    }

    failures
}

fn try_parse_cargo_test(raw: &str, max_lines: usize) -> Option<String> {
    // Must have a "test result:" line
    let summary_line = raw.lines().find(|l| l.starts_with("test result:"))?;

    let failures = extract_failure_blocks(raw);

    let mut output: Vec<String> = Vec::new();
    output.push(summary_line.to_string());

    if !failures.is_empty() {
        output.push(String::new());
        output.push("Failures:".to_string());
        for failure in &failures {
            output.push(format!("  {} — {}", failure.test_name, failure.message));
            if output.len() >= max_lines.saturating_sub(1) {
                break;
            }
        }
    }

    Some(output.join("\n"))
}

fn try_parse_pytest(raw: &str, max_lines: usize) -> Option<String> {
    // Must have "test session starts" and a summary line with passed/failed
    if !raw.contains("test session starts") {
        return None;
    }

    // Find summary line (contains "passed" or "failed" inside === delimiters)
    let summary_line = raw.lines().find(|l| {
        (l.contains("passed") || l.contains("failed")) && l.starts_with("=")
    })?;

    let mut output: Vec<String> = Vec::new();
    output.push(summary_line.trim_matches('=').trim().to_string());

    // Extract FAILED lines with their error messages
    let mut failures: Vec<(String, String)> = Vec::new();
    for line in raw.lines() {
        if line.starts_with("FAILED ") {
            let rest = line.trim_start_matches("FAILED ").trim();
            if let Some((test_name, error)) = rest.split_once(" - ") {
                failures.push((test_name.to_string(), error.to_string()));
            } else {
                failures.push((rest.to_string(), String::new()));
            }
        }
    }

    if !failures.is_empty() {
        output.push(String::new());
        output.push("Failures:".to_string());
        for (name, msg) in &failures {
            if msg.is_empty() {
                output.push(format!("  {}", name));
            } else {
                output.push(format!("  {} — {}", name, msg));
            }
            if output.len() >= max_lines.saturating_sub(1) {
                break;
            }
        }
    }

    Some(output.join("\n"))
}

pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Try to detect and parse specific formats
    if let Some(result) = try_parse_cargo_test(raw, max_lines) {
        return result;
    }
    if let Some(result) = try_parse_pytest(raw, max_lines) {
        return result;
    }

    // Fallback to generic
    super::generic::compress(raw, max_lines)
}
