//! Command status types for engine results.

use serde::{Deserialize, Serialize};

/// The outcome status of a command execution.
///
/// Maps to the exit code contract:
/// - `Success` → exit 0
/// - `Findings` → exit 1 (blocking findings detected)
/// - `UsageError` → exit 2
/// - `DeckExhausted` → exit 3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Success,
    Findings,
    UsageError,
    DeckExhausted,
}

impl CommandStatus {
    /// Convert to the documented exit code.
    pub fn exit_code(self) -> i32 {
        match self {
            CommandStatus::Success => 0,
            CommandStatus::Findings => 1,
            CommandStatus::UsageError => 2,
            CommandStatus::DeckExhausted => 3,
        }
    }

    /// Parse from an exit code. Returns None for unrecognized codes.
    pub fn from_exit_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(CommandStatus::Success),
            1 => Some(CommandStatus::Findings),
            2 => Some(CommandStatus::UsageError),
            3 => Some(CommandStatus::DeckExhausted),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_contract() {
        assert_eq!(CommandStatus::Success.exit_code(), 0);
        assert_eq!(CommandStatus::Findings.exit_code(), 1);
        assert_eq!(CommandStatus::UsageError.exit_code(), 2);
        assert_eq!(CommandStatus::DeckExhausted.exit_code(), 3);
    }

    #[test]
    fn from_exit_code_round_trips() {
        for status in [
            CommandStatus::Success,
            CommandStatus::Findings,
            CommandStatus::UsageError,
            CommandStatus::DeckExhausted,
        ] {
            let code = status.exit_code();
            assert_eq!(CommandStatus::from_exit_code(code), Some(status));
        }
    }

    #[test]
    fn from_exit_code_rejects_unknown() {
        assert_eq!(CommandStatus::from_exit_code(4), None);
        assert_eq!(CommandStatus::from_exit_code(-1), None);
        assert_eq!(CommandStatus::from_exit_code(127), None);
    }

    #[test]
    fn serializes_as_snake_case() {
        let json = serde_json::to_string(&CommandStatus::UsageError).unwrap();
        assert_eq!(json, "\"usage_error\"");
    }

    #[test]
    fn deserializes_from_snake_case() {
        let s: CommandStatus = serde_json::from_str("\"deck_exhausted\"").unwrap();
        assert_eq!(s, CommandStatus::DeckExhausted);
    }
}
