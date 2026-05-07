use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInvalidationEvent {
    pub path: String,
    pub reason: String,
}

pub struct CacheInvalidation;

impl CacheInvalidation {
    pub fn should_reindex(old_hash: &str, new_hash: &str) -> bool {
        old_hash != new_hash
    }

    pub fn invalidate(path: impl Into<String>, reason: impl Into<String>) -> CacheInvalidationEvent {
        CacheInvalidationEvent {
            path: path.into(),
            reason: reason.into(),
        }
    }
}
