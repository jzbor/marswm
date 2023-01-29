use core::result::Result as CoreResult;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::common::x11::atoms::X11Atom;

pub type Result<T> = CoreResult<T, MarsError>;

pub struct MarsError {
    kind: MarsErrorKind,
    info: Option<String>,
}

#[derive(Debug,Copy,Clone)]
pub enum MarsErrorKind {
    ConnectionFailed,
    IllegalValue,
    Unknown,
    UnsupportedProtocol,
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

impl Display for MarsErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let name = match self {
            Self::ConnectionFailed => "Connection failed",
            Self::IllegalValue => "Illegal value",
            Self::Unknown => "Unknown error",
            Self::UnsupportedProtocol => "Protocol not supported",
        };
        return write!(f, "{}", name);
    }
}


pub fn error_invalid_response(request: &(impl Display + ?Sized)) -> MarsError {
    return MarsError {
        kind: MarsErrorKind::IllegalValue,
        info: Some(format!("invalid response value to request '{}'", request)),
    };
}

pub fn error_failed_conversion(value: &(impl Display + ?Sized), from: &str, to: &str) -> MarsError {
    return MarsError {
        kind: MarsErrorKind::IllegalValue,
        info: Some(format!("unable to convert {} from {} to {}", value, from, to)),
    };
}

pub fn error_unknown(info: impl ToString) -> MarsError {
    return MarsError {
        kind: MarsErrorKind::Unknown,
        info: Some(info.to_string()),
    };
}

pub fn error_x11_unsupported_atom(atom: X11Atom) -> MarsError {
    return MarsError {
        kind: MarsErrorKind::UnsupportedProtocol,
        info: Some(atom.to_string()),
    };
}

pub fn error_x11_open_display() -> MarsError {
    return MarsError {
        kind: MarsErrorKind::ConnectionFailed,
        info: Some("XOpenDisplay".to_owned()),
    };
}
