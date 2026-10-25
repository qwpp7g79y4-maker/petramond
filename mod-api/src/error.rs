use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    WrongSide,
    NotInInit,
    NoContext,
    ReadOnly,
    NoActor,
    Forbidden,
    InvalidArgument,
    LimitExceeded,
    Refused,
}

impl ErrorCode {
    pub const fn is_recoverable(self) -> bool {
        matches!(self, Self::LimitExceeded | Self::Refused)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub code: ErrorCode,
    pub detail: String,
}

impl HostError {
    pub fn new(code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for HostError {}

impl From<HostError> for String {
    fn from(error: HostError) -> Self {
        error.detail
    }
}
