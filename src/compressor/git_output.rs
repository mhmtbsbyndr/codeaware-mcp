pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Detect diff output
    if raw.contains("diff --git") {
        return compress_diff(raw, max_lines);
    }

    // For log or status: just truncate with header
    let kept = &lines[..max_lines.saturating_sub(1)];
    let remaining = lines.len() - kept.len();
    let mut result: Vec<String> = kept.iter().map(|s| s.to_string()).collect();
    if remaining > 0 {
        result.push(format!("... {} more lines", remaining));
    }
    result.join("\n")
}

fn compress_diff(raw: &str, max_lines: usize) -> String {
    // Collect per-file summaries: file path, hunks added, hunks removed
    struct FileDiff {
        path: String,
        added: usize,
        removed: usize,
        hunks: usize,
    }

    let mut files: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    // True between an "@@" hunk header and the next "diff --git". Only lines
    // outside hunks can be "--- a/..." / "+++ b/..." file headers; inside a
    // hunk, "---"/"+++" prefixes are real content (e.g. "-- foo" -> "--- foo").
    let mut in_hunk = false;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            if let Some(fd) = current.take() {
                files.push(fd);
            }
            in_hunk = false;
            // Extract path: "diff --git a/src/foo.rs b/src/foo.rs" -> "src/foo.rs"
            let path = line
                .split_whitespace()
                .last()
                .unwrap_or("?")
                .trim_start_matches("b/")
                .to_string();
            current = Some(FileDiff { path, added: 0, removed: 0, hunks: 0 });
        } else if line.starts_with("@@") {
            in_hunk = true;
            if let Some(ref mut fd) = current {
                fd.hunks += 1;
            }
        } else if !in_hunk {
            // File header lines (---, +++, index, mode, ...): not counted.
        } else if line.starts_with('+') {
            if let Some(ref mut fd) = current {
                fd.added += 1;
            }
        } else if line.starts_with('-') {
            if let Some(ref mut fd) = current {
                fd.removed += 1;
            }
        }
    }
    if let Some(fd) = current {
        files.push(fd);
    }

    // Build summary lines: one per file
    let mut result: Vec<String> = Vec::new();
    for fd in &files {
        let summary = format!(
            "{}: +{} -{} ({} hunk{})",
            fd.path,
            fd.added,
            fd.removed,
            fd.hunks,
            if fd.hunks == 1 { "" } else { "s" }
        );
        result.push(summary);
        if result.len() >= max_lines {
            break;
        }
    }

    result.join("\n")
}
