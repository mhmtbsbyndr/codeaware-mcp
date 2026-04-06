use regex::Regex;

pub struct RegexFallback {
    patterns: Vec<(Regex, &'static str)>,
}

/// Simple struct for regex-extracted symbols.
pub struct RegexSymbol {
    pub name: String,
    pub kind: String,
    pub line: usize,
}

/// Compile a regex pattern with a descriptive panic message on failure.
fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("RegexFallback: invalid pattern {pattern:?}: {e}"))
}

impl RegexFallback {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                (re(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)"), "function"),
                (re(r"(?m)^(?:pub\s+)?struct\s+(\w+)"), "struct"),
                (re(r"(?m)^(?:pub\s+)?enum\s+(\w+)"), "enum"),
                (re(r"(?m)^(?:pub\s+)?trait\s+(\w+)"), "trait"),
                (re(r"(?m)^def\s+(\w+)"), "function"),
                (re(r"(?m)^class\s+(\w+)"), "class"),
                (re(r"(?m)^(?:export\s+)?function\s+(\w+)"), "function"),
                (re(r"(?m)^(?:export\s+)?class\s+(\w+)"), "class"),
                (re(r"(?m)^(?:export\s+)?interface\s+(\w+)"), "interface"),
            ],
        }
    }

    pub fn extract_symbols(&self, code: &str) -> Vec<RegexSymbol> {
        let mut symbols = Vec::new();
        for (regex, kind) in &self.patterns {
            for cap in regex.captures_iter(code) {
                if let Some(name) = cap.get(1) {
                    let line = code[..name.start()].lines().count();
                    symbols.push(RegexSymbol {
                        name: name.as_str().to_string(),
                        kind: kind.to_string(),
                        line,
                    });
                }
            }
        }
        // Sort by line, then deduplicate by name
        symbols.sort_by(|a, b| a.line.cmp(&b.line));
        symbols.dedup_by(|a, b| a.name == b.name);
        symbols
    }
}

impl Default for RegexFallback {
    fn default() -> Self {
        Self::new()
    }
}
