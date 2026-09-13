//! Structured errors for the Considered engine.

use thiserror::Error;

/// Top-level error type for engine operations.
///
/// All errors are explicit and structured — panics in ordinary invalid input
/// are defects per the implementation standards.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid severity: {0}")]
    InvalidSeverity(String),

    #[error("invalid artifact path: {0}")]
    InvalidPath(String),

    #[error("usage error: {0}")]
    Usage(String),

    #[error("unknown command: {0}")]
    UnknownCommand(String),

    #[error("{0}")]
    Other(String),
}

/// Map engine errors to process exit codes.
///
/// Exit codes match the documented contract:
/// - 0: success or clean
/// - 1: findings or failed gate
/// - 2: usage/input error
/// - 3: exhausted roll deck
impl EngineError {
    pub fn exit_code(&self) -> i32 {
        match self {
            EngineError::Usage(_) | EngineError::UnknownCommand(_) => 2,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_map_to_exit_2() {
        assert_eq!(EngineError::Usage("bad input".into()).exit_code(), 2);
        assert_eq!(EngineError::UnknownCommand("foo".into()).exit_code(), 2);
    }

    #[test]
    fn finding_errors_map_to_exit_1() {
        assert_eq!(EngineError::InvalidSeverity("X".into()).exit_code(), 1);
        assert_eq!(EngineError::InvalidPath("/abs".into()).exit_code(), 1);
    }

    #[test]
    fn error_display_is_human_readable() {
        let e = EngineError::Usage("missing file".into());
        assert_eq!(e.to_string(), "usage error: missing file");
    }
}
