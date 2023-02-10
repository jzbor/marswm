use core::result::Result as CoreResult;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::common::x11::atoms::X11Atom;

pub type Result<T> = CoreResult<T, MarsError>;

#[derive(Debug,Clone)]
pub struct MarsError {
    kind: MarsErrorKind,
    info: Option<String>,
}

#[derive(Debug,Copy,Clone)]
pub enum MarsErrorKind {
    ConnectionFailed,
    IllegalValue,
    FailedRequest,
    PropertyUnavailable,
    Unknown,
    UnsupportedProtocol,
}


impl MarsError {
    pub fn failed_request(call: &str) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::FailedRequest,
            info: Some(format!("{} failed", call)),
        }
    }

    pub fn failed_conversion(value: impl std::fmt::Debug, from: &str, to: &str) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::IllegalValue,
            info: Some(format!("unable to convert {:?} from {} to {}", value, from, to)),
        };
    }

    pub fn invalid_response(request: impl Display) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::IllegalValue,
            info: Some(format!("invalid response value to request '{}'", request)),
        };
    }

    pub fn property_unavailable(property: impl ToString) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::PropertyUnavailable,
            info: Some(property.to_string()),
        };
    }

    pub fn unknown(info: impl ToString) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::Unknown,
            info: Some(info.to_string()),
        };
    }

    pub fn x11_unsupported_atom(atom: X11Atom) -> MarsError {
        return MarsError {
            kind: MarsErrorKind::UnsupportedProtocol,
            info: Some(atom.to_string()),
        };
    }

    pub fn x11_open_display() -> MarsError {
        return MarsError {
            kind: MarsErrorKind::ConnectionFailed,
            info: Some("XOpenDisplay".to_owned()),
        };
    }
}


impl Display for MarsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.kind.fmt(f)?;
        if let Some(info) = &self.info {
            write!(f, " ({})", info)?;
        }
        return Ok(());
    }
}

impl From<MarsError> for String {
    fn from(value: MarsError) -> Self {
        return value.to_string();
    }
}

impl Display for MarsErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let name = match self {
            Self::ConnectionFailed => "Connection failed",
            Self::IllegalValue => "Illegal value",
            Self::Unknown => "Unknown error",
            Self::UnsupportedProtocol => "Protocol not supported",
            Self::PropertyUnavailable => "Property not available",
            Self::FailedRequest => "Failed request",
        };
        return write!(f, "{}", name);
    }
}


