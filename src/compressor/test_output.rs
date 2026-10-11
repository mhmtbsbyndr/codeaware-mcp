use super::{compiler_output, generic};

/// One failed test: its name and the lines that explain why it failed.
struct FailureBlock {
    test_name: String,
    /// First entry is the panic line (kept verbatim minus the thread name), then up to two message lines.
    detail: Vec<String>,
}

/// Drop the redundant "thread '<name>' " prefix so the panic location stays visible.
fn panic_line_without_thread(line: &str) -> String {
    match line.find("panicked at") {
        Some(pos) => line[pos..].trim().to_string(),
        None => line.trim().to_string(),
    }
}

fn is_noise(line: &str) -> bool {
    line.trim().is_empty() || line.trim_start().starts_with("note:")
}

fn detail_for_block(body: &[&str]) -> Vec<String> {
    if let Some(pos) = body.iter().position(|l| l.contains("panicked at")) {
        let mut detail = vec![panic_line_without_thread(body[pos])];
        detail.extend(
            body[pos + 1..]
                .iter()
                .filter(|l| !is_noise(l))
                .take(2)
                .map(|l| l.trim().to_string()),
        );
        return detail;
    }
    // No panic line: keep the first meaningful lines of the block.
    body.iter()
        .filter(|l| !is_noise(l))
        .take(3)
        .map(|l| l.trim().to_string())
        .collect()
}

fn extract_failure_blocks(lines: &[&str]) -> Vec<FailureBlock> {
    let mut failures = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        // cargo test: "---- <test_name> stdout ----"
        if lines[i].starts_with("---- ") && lines[i].ends_with(" stdout ----") {
            let test_name = lines[i]
                .trim_start_matches("---- ")
                .trim_end_matches(" stdout ----")
                .to_string();
            let start = i + 1;
            i += 1;
            while i < lines.len()
                && !lines[i].starts_with("---- ")
                && !lines[i].starts_with("failures:")
                && !lines[i].starts_with("test result:")
            {
                i += 1;
            }
            let detail = detail_for_block(&lines[start..i]);
            failures.push(FailureBlock { test_name, detail });
        } else {
            i += 1;
        }
    }
    failures
}

/// Names of tests listed in the cargo "failures:" summary that have no stdout block.
fn extract_failure_names_only(lines: &[&str], known: &[FailureBlock]) -> Vec<FailureBlock> {
    let mut extra = Vec::new();
    let mut in_list = false;
    for line in lines {
        if line.trim() == "failures:" {
            in_list = true;
            continue;
        }
        if in_list {
            if line.trim().is_empty() || line.starts_with("test result:") {
                if line.starts_with("test result:") {
                    in_list = false;
                }
                continue;
            }
            if line.starts_with("    ") {
                let name = line.trim().to_string();
                if !known.iter().any(|f| f.test_name == name) && !extra.iter().any(|f: &FailureBlock| f.test_name == name) {
                    extra.push(FailureBlock { test_name: name, detail: Vec::new() });
                }
            } else {
                in_list = false;
            }
        }
    }
    extra
}

/// Assemble summary lines plus one line per failed test.
/// Failing names are never dropped; detail lines are shortened when the budget is tight.
fn render_failures(summaries: &[&str], failures: &[FailureBlock], max_lines: usize) -> String {
    let mut out: Vec<String> = summaries.iter().map(|s| s.to_string()).collect();
    if !failures.is_empty() {
        out.push(String::new());
        out.push("Failures:".to_string());
        let full: Vec<String> = failures
            .iter()
            .map(|f| {
                if f.detail.is_empty() {
                    format!("  {}", f.test_name)
                } else {
                    format!("  {} — {}", f.test_name, f.detail.join(" | "))
                }
            })
            .collect();
        if out.len() + full.len() <= max_lines {
            out.extend(full);
        } else {
            // Compact form: name plus the panic location only.
            out.extend(failures.iter().map(|f| match f.detail.first() {
                Some(first) => format!("  {} — {}", f.test_name, first),
                None => format!("  {}", f.test_name),
            }));
        }
    }
    out.join("\n")
}

fn try_parse_cargo_test(raw: &str, max_lines: usize) -> Option<String> {
    let lines: Vec<&str> = raw.lines().collect();
    // One "test result:" line per test binary; keep them all so counts stay complete.
    let summaries: Vec<&str> = lines.iter().copied().filter(|l| l.starts_with("test result:")).collect();
    if summaries.is_empty() {
        return None;
    }
    let mut failures = extract_failure_blocks(&lines);
    failures.extend(extract_failure_names_only(&lines, &failures));
    Some(render_failures(&summaries, &failures, max_lines))
}

fn try_parse_pytest(raw: &str, max_lines: usize) -> Option<String> {
    if !raw.contains("test session starts") {
        return None;
    }
    let summary_line = raw.lines().find(|l| {
        (l.contains("passed") || l.contains("failed")) && l.starts_with('=')
    })?;
    let summary = summary_line.trim_matches('=').trim();

    let mut failures: Vec<FailureBlock> = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("FAILED ") {
            let rest = rest.trim();
            let (name, error) = rest.split_once(" - ").unwrap_or((rest, ""));
            let detail = if error.is_empty() { Vec::new() } else { vec![error.to_string()] };
            failures.push(FailureBlock { test_name: name.to_string(), detail });
        }
    }
    Some(render_failures(&[summary], &failures, max_lines))
}

/// Compile errors that stopped the test run before any test ran.
fn is_compile_failure(raw: &str) -> bool {
    raw.lines().any(|l| l.starts_with("error[E") || l.starts_with("error: could not compile"))
}

pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    if let Some(result) = try_parse_cargo_test(raw, max_lines) {
        return result;
    }
    if let Some(result) = try_parse_pytest(raw, max_lines) {
        return result;
    }
    if is_compile_failure(raw) {
        return compiler_output::compress(raw, max_lines);
    }

    generic::compress(raw, max_lines)
}
