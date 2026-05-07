use thiserror::Error;

pub type V4Result<T> = Result<T, V4Error>;

#[derive(Debug, Error)]
pub enum V4Error {
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("contract violation: {0}")]
    ContractViolation(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("context assembly failed: {0}")]
    ContextAssembly(String),
}
