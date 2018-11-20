extern crate libc;

use self::libc::{c_int, strerror, c_char};
use self::libc::unix::solaris::*;

#[derive(Debug)]
pub struct Errno {
    code: c_int,
    text: String,
}

impl Errno {
    pub fn last() -> Self {
        let errno_ptr: *const c_int = unsafe {
            if cfg!(target_os = "solaris") {
                self::libc::___errno()
            } else {
                self::libc::__errno_location()
            }
        };
        if errno_ptr.is_null() {
            ::write_exit(1, "errno location is unknown");
        } else {
            let code: c_int = unsafe { *errno_ptr };
            let text: *const c_char = unsafe { strerror(code) };
            if text.is_null() {
                ::write_exit(2, "errno code is unknown");
            } else {
                if let Ok(text) = unsafe { ::copy_string(text) } {
                    Errno { code, text }
                } else {
                    ::write_exit(3, "errno string is incorrect C string");
                }
            }
        }
    }
}
