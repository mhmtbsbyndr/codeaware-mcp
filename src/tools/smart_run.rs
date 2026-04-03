use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;
use regex::Regex;

const TIMEOUT_SECS: u64 = 120;
const MAX_OUTPUT_BYTES: usize = 100 * 1024;

pub struct SmartRunInput {
    pub command: String,
    pub max_output_lines: usize,
    pub capture_relevant_code: bool,
    pub scan_secrets: bool,
}

impl Default for SmartRunInput {
    fn default() -> Self {
        SmartRunInput {
            command: String::new(),
            max_output_lines: 50,
            capture_relevant_code: true,
            scan_secrets: true,
        }
    }
}

pub struct SmartRunResult {
    pub command: String,
    pub command_type: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub raw_lines: usize,
    pub compressed_lines: usize,
    pub compression_ratio: f64,
    pub summary: String,
    pub compressed_output: String,
    pub failures: Vec<Value>,
    pub warnings: Vec<String>,
    pub secrets_detected: bool,
    pub error_recurrence: Value,
    pub suggested_next: Vec<String>,
}

#[derive(Debug)]
pub enum SmartRunError {
    Timeout(u64),
    Io(std::io::Error),
    Denied(String),
}

impl std::fmt::Display for SmartRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmartRunError::Timeout(secs) => write!(f, "Command timed out after {}s", secs),
            SmartRunError::Io(e) => write!(f, "IO error: {}", e),
            SmartRunError::Denied(msg) => write!(f, "Denied: {}", msg),
        }
    }
}

impl std::error::Error for SmartRunError {}

impl From<std::io::Error> for SmartRunError {
    fn from(e: std::io::Error) -> Self {
        SmartRunError::Io(e)
    }
}

/// Computes a deterministic signature for error output.
/// Returns empty string if no errors detected.
pub fn compute_error_signature(output: &str) -> String {
    // Extract error-relevant lines
    let error_lines: Vec<&str> = output.lines()
        .filter(|l| {
            let trimmed = l.trim().to_lowercase();
            // Match lines starting with "error" or containing "panic"
            // For "failed", only match if it's not part of a success message (0 failed)
            
            trimmed.starts_with("error")
                || trimmed.contains("panic")
                || (trimmed.contains("failed") && !trimmed.contains("0 failed"))
        })
        .collect();

    if error_lines.is_empty() {
        return String::new();
    }

    // Normalize: strip timestamps, line numbers, paths that may change
    let normalized: String = error_lines.iter()
        .map(|l| {
            // Strip ISO timestamps (e.g., 2026-04-01T12:00:00Z)
            let re_ts = Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[Z\w]*").unwrap();
            let stripped = re_ts.replace_all(l, "<TS>");
            stripped.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Hash the normalized error text
    blake3::hash(normalized.as_bytes()).to_hex()[..16].to_string()
}

pub async fn smart_run(input: &SmartRunInput) -> Result<SmartRunResult, SmartRunError> {
    let command_type = crate::compressor::classify_command(&input.command);

    let start = Instant::now();

    let run_future = Command::new("sh")
        .arg("-c")
        .arg(&input.command)
        .output();

    let output = match timeout(Duration::from_secs(TIMEOUT_SECS), run_future).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(SmartRunError::Io(e)),
        Err(_) => return Err(SmartRunError::Timeout(TIMEOUT_SECS)),
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);

    // Combine stdout + stderr
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    // Truncate if > 100KB
    if combined.len() > MAX_OUTPUT_BYTES {
        let mut boundary = MAX_OUTPUT_BYTES;
        while !combined.is_char_boundary(boundary) {
            boundary -= 1;
        }
        combined.truncate(boundary);
    }

    // Secret scanning
    let (processed_output, secrets_detected) = if input.scan_secrets {
        let scanner = crate::security::secret_scanner::SecretScanner::new();
        scanner.scan(&combined)
    } else {
        (combined.clone(), false)
    };

    // Count raw lines
    let raw_lines = processed_output.lines().count();

    // Compress
    let compressed_output =
        crate::compressor::compress_output(command_type, &processed_output, input.max_output_lines);
    let compressed_lines = compressed_output.lines().count();

    // Compression ratio
    let compression_ratio = if raw_lines > 0 {
        compressed_lines as f64 / raw_lines as f64
    } else {
        1.0
    };

    // Summary: first non-empty line or a default message
    let summary = compressed_output
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no output)")
        .to_string();

    // Compute error signature
    let error_sig = compute_error_signature(&combined);
    let error_recurrence = if error_sig.is_empty() {
        Value::Null
    } else {
        json!({
            "signature": error_sig
        })
    };

    Ok(SmartRunResult {
        command: input.command.clone(),
        command_type: command_type.to_string(),
        exit_code,
        duration_ms,
        raw_lines,
        compressed_lines,
        compression_ratio,
        summary,
        compressed_output,
        failures: vec![],
        warnings: vec![],
        secrets_detected,
        error_recurrence,
        suggested_next: vec![],
    })
}
