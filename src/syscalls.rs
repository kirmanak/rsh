extern crate libc;

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

pub type Uid = u32;

/// Gets Uid of the current user.
pub fn get_uid() -> Uid {
    unsafe { libc::getuid() }
}

/// Gets gid of the current user.
pub fn get_gid() -> Uid {
    unsafe { libc::getgid() }
}

/// Gets user's home directory from the corresponding record in passwd.
pub fn get_home_dir(uid: Uid) -> Option<PathBuf> {
    let pwd_entry: *mut libc::passwd = unsafe { libc::getpwuid(uid) };
    if pwd_entry.is_null() {
        None
    } else {
        let dir: *mut libc::c_char = unsafe { (*pwd_entry).pw_dir };
        if dir.is_null() {
            None
        } else {
            let len = unsafe { libc::strlen(dir) };
            let result = unsafe {
                String::from_raw_parts(dir as *mut u8, len, len)
            };
            let result = PathBuf::from(result);
            Some(result)
        }
    }
}