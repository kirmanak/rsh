use std::fmt::{Formatter, Display};
use super::libc::{c_int, strerror, c_char};

use super::{write_exit, copy_string};

/// Forces usage of rsh::native::Error in Results
pub type Result<T> = std::result::Result<T, Error>;

/// Represents all possible errors in this program.
#[derive(Debug)]
pub enum Error {
    InvalidCString,
    InvalidUnicode,
    NotFound,
    Errno(Errno),
}

impl Error {
    pub fn from_errno() -> Self {
        Error::Errno(Errno::last())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidCString => write!(formatter, "Fail to produce valid C string"),
            Error::InvalidUnicode => write!(formatter, "Fail to produce valid Unicode string"),
            Error::NotFound => write!(formatter, "Value was not found"),
            Error::Errno(reason) => write!(formatter, "{}", reason),
        }
    }
}

/// Wraps errno state and gets the description from the system
#[derive(Debug)]
pub struct Errno {
    code: c_int,
    text: String,
}

#[cfg(target_os = "solaris")]
unsafe fn errno() -> *const c_int {
    super::libc::___errno()
}

#[cfg(not(target_os = "solaris"))]
unsafe fn errno() -> *const c_int {
    super::libc::__errno_location()
}

impl Errno {
    /// Wraps the current state of errno
    pub fn last() -> Self {
        let errno_ptr: *const c_int = unsafe { errno() };
        if errno_ptr.is_null() {
            write_exit(1, "errno location is unknown");
        } else {
            let code: c_int = unsafe { *errno_ptr };
            let text: *const c_char = unsafe { strerror(code) };
            if text.is_null() {
                write_exit(2, "errno code is unknown");
            } else {
                if let Ok(text) = unsafe { copy_string(text) } {
                    Errno { code, text }
                } else {
                    write_exit(3, "errno string is incorrect C string");
                }
            }
        }
    }
}

impl Display for Errno {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "{}", &self.text)
    }
}
