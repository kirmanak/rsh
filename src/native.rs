extern crate libc;
extern crate nix;

use std::ffi::CStr;

use nix::{Error, Result};
use nix::unistd::Uid;

use self::libc::{c_char, getpwuid, passwd, strcpy, strlen};

/// Gets user's home directory from the corresponding record in /etc/passwd.
pub fn get_home_dir(uid: Uid) -> Result<String> {
    let passwd_ptr: *mut passwd = unsafe { getpwuid(uid.into()) };
    if !passwd_ptr.is_null() {
        let dir: *mut c_char = unsafe { (*passwd_ptr).pw_dir };
        if !dir.is_null() {
            let string = unsafe { copy_string(dir) };
            let string = String::from(string.to_string_lossy());
            Ok(string)
        } else {
            Err(Error::UnsupportedOperation)
        }
    } else {
        Err(Error::last())
    }
}

/// When Rust tries to manage the memory which was allocated by something else, the program falls with segfault
unsafe fn copy_string<'a>(native: *mut c_char) -> &'a CStr {
    let buf = vec![0; strlen(native)].as_mut_ptr();
    strcpy(buf, native);
    CStr::from_ptr(buf)
}