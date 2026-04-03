pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Severity {
        Error = 0,
        Warning = 1,
        Suggestion = 2,
    }

    struct Issue {
        severity: Severity,
        block: String,
    }

    let mut issues: Vec<Issue> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        let severity = if line.starts_with("error:") || line.starts_with("error[") {
            Some(Severity::Error)
        } else if line.starts_with("warning:") {
            Some(Severity::Warning)
        } else if line.starts_with("suggestion:") || line.starts_with("note:") || line.starts_with("help:") {
            Some(Severity::Suggestion)
        } else {
            None
        };

        if let Some(sev) = severity {
            let mut block = vec![line.to_string()];
            i += 1;
            // Grab location line
            if i < lines.len() && lines[i].trim_start().starts_with("-->") {
                block.push(lines[i].to_string());
                i += 1;
            }
            issues.push(Issue { severity: sev, block: block.join("\n") });
            continue;
        }

        i += 1;
    }

    // Sort by severity (errors first)
    issues.sort_by(|a, b| a.severity.cmp(&b.severity));

    let mut result: Vec<String> = Vec::new();
    let mut used = 0usize;

    for issue in &issues {
        let lc = issue.block.lines().count();
        if used + lc <= max_lines {
            result.push(issue.block.clone());
            used += lc;
        } else {
            break;
        }
    }

    result.join("\n")
}
