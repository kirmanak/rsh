extern crate libc;

use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Option<String> {
    let capacity = 256; // according to man, any host name is limited to 256 characters
    let mut buf = Vec::with_capacity(capacity);
    let status = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut i8, capacity) };
    if status == 0 {
        let len = unsafe { libc::strlen(buf.as_ptr() as *const i8) };
        unsafe { buf.set_len(len); }
        String::from_utf8(buf).ok()
    } else {
        None
    }
}

pub type UserId = u32;
pub type GroupId = u32;

/// Gets Uid of the current user.
pub fn get_uid() -> UserId {
    unsafe { libc::getuid() }
}

/// Gets gid of the current user.
pub fn get_gid() -> GroupId {
    unsafe { libc::getgid() }
}

/// Gets user's home directory from the corresponding record in passwd.
pub fn get_home_dir(uid: UserId) -> Option<PathBuf> {
    let pwd_entry: *mut libc::passwd = unsafe { libc::getpwuid(uid) };
    if pwd_entry.is_null() {
        None
    } else {
        let dir: *mut libc::c_char = unsafe { (*pwd_entry).pw_dir };
        if dir.is_null() {
            None
        } else {
            let path = unsafe { wrap_string(dir) };
            let path = PathBuf::from(path);
            Some(path)
        }
    }
}

pub type FileDescriptorId = i32;

pub fn open_file(path: &str, flags: i32) -> Result<FileDescriptorId> {
    let status = unsafe { libc::open(path.as_ptr() as *const i8, flags) };
    if status < 0 {
        let error_text = unsafe {
            let errno = *libc::__errno_location();
            wrap_string(libc::strerror(errno))
        };
        let kind = unsafe { map_errno() };
        Err(Error::new(kind, error_text.as_str()))
    } else {
        Ok(status)
    }
}

unsafe fn wrap_string(ptr: *mut i8) -> String {
    let len = libc::strlen(ptr);
    String::from_raw_parts(ptr as *mut u8, len, len)
}

unsafe fn map_errno() -> ErrorKind {
    let errno = *libc::__errno_location();
    match errno {
        1 => ErrorKind::PermissionDenied,
        2 => ErrorKind::NotFound,
        4 => ErrorKind::Interrupted,
        _ => ErrorKind::Other,
    }
}