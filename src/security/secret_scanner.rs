use regex::Regex;

const MAX_SCAN_BYTES: usize = 100 * 1024;

pub struct SecretPattern {
    pub label: String,
    pub regex: Regex,
}

pub struct SecretScanner {
    pub patterns: Vec<SecretPattern>,
}

impl SecretScanner {
    pub fn new() -> Self {
        let raw_patterns: &[(&str, &str)] = &[
            ("api_key",      r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[\w-]{20,}"#),
            ("secret",       r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"]?[\w-]{8,}"#),
            ("token",        r#"(?i)(token|bearer)\s*[:=]\s*['"]?[\w.\-]{20,}"#),
            ("github_token", r"(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}"),
            ("openai_key",   r"sk-[A-Za-z0-9]{20,}"),
            ("private_key",  r"-----BEGIN (RSA |EC )?PRIVATE KEY-----"),
            ("aws_key",      r"AKIA[0-9A-Z]{16}"),
            ("aws_secret",   r"(?i)aws.{0,10}secret.{0,10}=.{0,5}[A-Za-z0-9+/]{40}"),
            ("anthropic_key",r"sk-ant-[A-Za-z0-9_-]{40,}"),
            ("jwt",          r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"),
            ("stripe_live",  r"sk_live_[A-Za-z0-9]{24,}"),
            ("stripe_test",  r"sk_test_[A-Za-z0-9]{24,}"),
            ("password_url", r"[a-zA-Z][a-zA-Z0-9+\-.]+://[^:@\s]+:[^@\s]{8,}@"),
            ("twilio_sid",   r"AC[a-f0-9]{32}"),
        ];

        let patterns = raw_patterns
            .iter()
            .map(|(label, pattern)| SecretPattern {
                label: label.to_string(),
                regex: Regex::new(pattern).expect("invalid secret pattern"),
            })
            .collect();

        SecretScanner { patterns }
    }

    /// Returns the number of patterns loaded.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Scans up to MAX_SCAN_BYTES of `text`.
    /// Returns `(redacted_text, secrets_detected)`.
    pub fn scan(&self, text: &str) -> (String, bool) {
        // Truncate to MAX_SCAN_BYTES at a char boundary
        let scan_text = if text.len() > MAX_SCAN_BYTES {
            let mut boundary = MAX_SCAN_BYTES;
            while !text.is_char_boundary(boundary) {
                boundary -= 1;
            }
            &text[..boundary]
        } else {
            text
        };

        let mut result = scan_text.to_string();
        let mut detected = false;

        for pattern in &self.patterns {
            if pattern.regex.is_match(&result) {
                detected = true;
                let label = format!("[REDACTED:{}]", pattern.label);
                result = pattern.regex.replace_all(&result, label.as_str()).into_owned();
            }
        }

        (result, detected)
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}
