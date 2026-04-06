use chrono::Utc;
use serde::{Serialize, Serializer};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Envelope<T: Serialize> {
    pub ok: bool,
    pub error_code: Option<ErrorCode>,
    pub retryable: bool,
    pub fallback_suggestion: Option<String>,
    pub trust: TrustLevel,
    pub trace_id: String,
    pub data: Option<T>,
}

impl<T: Serialize> Envelope<T> {
    pub fn success(data: T, trust: TrustLevel) -> Self {
        Self {
            ok: true,
            error_code: None,
            retryable: false,
            fallback_suggestion: None,
            trust,
            trace_id: generate_trace_id(),
            data: Some(data),
        }
    }

    pub fn error(code: ErrorCode, retryable: bool, fallback: Option<String>) -> Self {
        Self {
            ok: false,
            error_code: Some(code),
            retryable,
            fallback_suggestion: fallback,
            trust: TrustLevel::Raw,
            trace_id: generate_trace_id(),
            data: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ErrorCode {
    #[serde(rename = "E_AMBIGUOUS_MATCH")]
    EAmbiguousMatch,
    #[serde(rename = "E_STALE_READ")]
    EStaleRead,
    #[serde(rename = "E_SYMLINK_ESCAPE")]
    ESymlinkEscape,
    #[serde(rename = "E_SECRET_BLOCKED")]
    ESecretBlocked,
    #[serde(rename = "E_LSP_UNAVAILABLE")]
    ELspUnavailable,
    #[serde(rename = "E_LSP_TIMEOUT")]
    ELspTimeout,
    #[serde(rename = "E_PARSE_FAILED")]
    EParseFailed,
    #[serde(rename = "E_BINARY_FILE")]
    EBinaryFile,
    #[serde(rename = "E_FILE_TOO_LARGE")]
    EFileTooLarge,
    #[serde(rename = "E_PERMISSION_DENIED")]
    EPermissionDenied,
    #[serde(rename = "E_SYNTAX_INVALID")]
    ESyntaxInvalid,
    #[serde(rename = "E_HASH_MISMATCH")]
    EHashMismatch,
    #[serde(rename = "E_COMMAND_DENIED")]
    ECommandDenied,
    #[serde(rename = "E_SQLITE_LOCKED")]
    ESqliteLocked,
    #[serde(rename = "E_MCP_VERSION_MISMATCH")]
    EMcpVersionMismatch,
    #[serde(rename = "E_INVALID_SLOT")]
    EInvalidSlot,
    #[serde(rename = "E_INVALID_SLOT_VALUE")]
    EInvalidSlotValue,
    #[serde(rename = "E_INTERNAL_ERROR")]
    EInternalError,
    #[serde(rename = "E_LOW_CONFIDENCE")]
    ELowConfidence,
    #[serde(rename = "E_GIT_NOT_FOUND")]
    EGitNotFound,
    #[serde(rename = "E_GIT_ERROR")]
    EGitError,
}

#[derive(Debug, Clone, Copy)]
pub enum TrustLevel {
    Exact,
    Structural,
    Heuristic,
    Degraded,
    Raw,
}

impl Serialize for TrustLevel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = match self {
            TrustLevel::Exact => "exact",
            TrustLevel::Structural => "structural",
            TrustLevel::Heuristic => "heuristic",
            TrustLevel::Degraded => "degraded",
            TrustLevel::Raw => "raw",
        };
        serializer.serialize_str(s)
    }
}

fn generate_trace_id() -> String {
    let now = Utc::now().format("%Y%m%d-%H%M");
    let short_uuid = &Uuid::new_v4().to_string()[..8];
    format!("t-{now}-{short_uuid}")
}
