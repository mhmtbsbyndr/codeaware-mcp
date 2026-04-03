pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Patterns to skip (noisy progress lines)
    let skip_prefixes = [
        "  Downloading ",
        "   Downloading ",
        "    Downloading ",
        " Downloading ",
        "  Fetching ",
        "   Fetching ",
        "    Fetching ",
        "  Unpacking ",
        "   Unpacking ",
        "  Locking ",
    ];

    // Patterns to always keep (summary / important)
    let keep_prefixes = [
        "error",
        "warning",
        "  Downloaded",
        "   Downloaded",
        "    Downloaded",
        " Downloaded",
        "  added ",
        "  Compiling",
        "   Compiling",
        "    Compiling",
        "  Finished",
        "   Finished",
        "    Finished",
        " Finished",
        "  Installing",
        "  Installed",
        "  Removed",
        "  Summary",
    ];

    let mut kept: Vec<String> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        let should_skip = skip_prefixes.iter().any(|p| line.starts_with(p));
        if should_skip {
            continue;
        }
        let _ = trimmed;
        kept.push(line.to_string());
    }

    // If still over budget, keep only keep_prefix lines
    if kept.len() > max_lines {
        kept = lines
            .iter()
            .filter(|l| keep_prefixes.iter().any(|p| l.trim_start().starts_with(p.trim_start())))
            .map(|s| s.to_string())
            .collect();
    }

    // Final truncation
    if kept.len() > max_lines {
        let remaining = kept.len() - max_lines;
        kept.truncate(max_lines);
        kept.push(format!("... {} more lines", remaining));
    }

    if kept.is_empty() {
        // Fallback: just show last max_lines
        lines[lines.len().saturating_sub(max_lines)..]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        kept.join("\n")
    }
}
