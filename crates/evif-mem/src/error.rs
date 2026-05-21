//! Error types for the memory platform

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Vector error: {0}")]
    Vector(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Workflow error: {0}")]
    WorkflowError(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),
}

pub type MemResult<T> = Result<T, MemError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem_error_not_found() {
        let err = MemError::NotFound("item-123".to_string());
        assert!(err.to_string().contains("Not found"));
        assert!(err.to_string().contains("item-123"));
    }

    #[test]
    fn test_mem_error_invalid_input() {
        let err = MemError::InvalidInput("invalid query".to_string());
        assert!(err.to_string().contains("Invalid input"));
    }

    #[test]
    fn test_mem_error_storage() {
        let err = MemError::Storage("database locked".to_string());
        assert!(err.to_string().contains("Storage"));
    }

    #[test]
    fn test_mem_error_embedding() {
        let err = MemError::Embedding("model unavailable".to_string());
        assert!(err.to_string().contains("Embedding"));
    }

    #[test]
    fn test_mem_error_llm() {
        let err = MemError::Llm("rate limit exceeded".to_string());
        assert!(err.to_string().contains("LLM"));
    }

    #[test]
    fn test_mem_error_vector() {
        let err = MemError::Vector("index corrupted".to_string());
        assert!(err.to_string().contains("Vector"));
    }

    #[test]
    fn test_mem_error_workflow() {
        let err = MemError::WorkflowError("step 3 failed".to_string());
        assert!(err.to_string().contains("Workflow"));
    }

    #[test]
    fn test_mem_error_security() {
        let err = MemError::Security("unauthorized access".to_string());
        assert!(err.to_string().contains("Security"));
    }

    #[test]
    fn test_mem_result_ok() {
        let result: MemResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_mem_result_err() {
        let result: MemResult<i32> = Err(MemError::NotFound("test".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MemError::NotFound(_)));
    }

    #[test]
    fn test_mem_error_debug() {
        let err = MemError::NotFound("debug-test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }
}
