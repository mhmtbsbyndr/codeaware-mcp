pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Collect formatted file names and summary
    let mut formatted_files: Vec<String> = Vec::new();
    let mut summary: Option<String> = None;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("Formatted ") || trimmed.starts_with("Reformatted ") {
            formatted_files.push(trimmed.to_string());
        } else if trimmed.contains("file") && (trimmed.contains("formatted") || trimmed.contains("changed")) {
            summary = Some(trimmed.to_string());
        }
    }

    let mut result: Vec<String> = Vec::new();
    let mut used = 0usize;

    // Show summary first if present
    if let Some(ref s) = summary {
        result.push(s.clone());
        used += 1;
    }

    // Show file names up to budget
    for f in &formatted_files {
        if used >= max_lines {
            break;
        }
        result.push(f.clone());
        used += 1;
    }

    if result.is_empty() {
        // Fallback: truncate
        lines[..max_lines].iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n")
    } else {
        result.join("\n")
    }
}
