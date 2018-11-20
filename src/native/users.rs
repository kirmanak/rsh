extern crate libc;

use std::path::PathBuf;
use self::libc::{passwd, getpwuid, getuid, getgid, c_char};

pub type UserId = u32;
pub type GroupId = u32;

/// Gets Uid of the current user.
pub fn get_uid() -> UserId {
    unsafe { getuid() }
}

/// Gets gid of the current user.
pub fn get_gid() -> GroupId {
    unsafe { getgid() }
}

/// Gets user's home directory from the corresponding record in passwd.
pub fn get_home_dir(uid: UserId) -> ::Result<PathBuf> {
    let entry: *const passwd = unsafe { getpwuid(uid) };
    if entry.is_null() {
        Err(::Error::from_errno())
    } else {
        let dir: *const c_char = unsafe { (*entry).pw_dir };
        if dir.is_null() {
            Err(::Error::NotFound)
        } else {
            let path = unsafe { ::copy_string(dir)? };
            Ok(PathBuf::from(path))
        }
    }
}
