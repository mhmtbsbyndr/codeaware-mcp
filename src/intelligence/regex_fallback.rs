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

impl RegexFallback {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                (Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap(), "function"),
                (Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)").unwrap(), "struct"),
                (Regex::new(r"(?m)^(?:pub\s+)?enum\s+(\w+)").unwrap(), "enum"),
                (Regex::new(r"(?m)^(?:pub\s+)?trait\s+(\w+)").unwrap(), "trait"),
                (Regex::new(r"(?m)^def\s+(\w+)").unwrap(), "function"),
                (Regex::new(r"(?m)^class\s+(\w+)").unwrap(), "class"),
                (Regex::new(r"(?m)^(?:export\s+)?function\s+(\w+)").unwrap(), "function"),
                (Regex::new(r"(?m)^(?:export\s+)?class\s+(\w+)").unwrap(), "class"),
                (Regex::new(r"(?m)^(?:export\s+)?interface\s+(\w+)").unwrap(), "interface"),
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
