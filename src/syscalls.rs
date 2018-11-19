extern crate libc;
extern crate nix;

use std::borrow::Cow;
use std::ffi::CStr;

use libc::{c_char, getpwuid, passwd, strcpy, strlen};
use nix::{Error, Result};
use nix::unistd::Uid;

/// Gets user's home directory from the corresponding record in /etc/passwd.
pub fn get_home_dir(uid: Uid) -> Result<String> {
    let passwd_ptr: *mut passwd = unsafe { getpwuid(uid) };
    if !passwd_ptr.is_null() {
        let dir: *mut c_char = unsafe { (*passwd_ptr).pw_dir };
        if !dir.is_null() {
            let string = unsafe { copy_string(dir) };
            let string = lossy_string(string);
            Ok(string)
        } else {
            Err(Error::UnsupportedOperation)
        }
    } else {
        Err(Error::last())
    }
}

/// Creates a Rust string from a C string replacing invalid UTF-8 characters with U+FFFD
pub fn lossy_string(native: &CStr) -> String {
    match native.to_string_lossy() {
        Cow::Borrowed(text) => String::from(text),
        Cow::Owned(text) => text
    }
}

/// When Rust tries to manage the memory which was allocated by something else, the program falls with segfault
unsafe fn copy_string<'a>(native: *mut c_char) -> &'a CStr {
    let buf = vec![0; strlen(native)].as_mut_ptr();
    strcpy(buf, native);
    CStr::from_ptr(buf)
}