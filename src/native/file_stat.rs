extern crate libc;

use std::path::PathBuf;

use self::libc::{stat, c_int, c_char};

use {Result, Error, UserId, GroupId};

/// Calls stat(2) on the file to determine an owner-user
pub fn get_file_uid(path: &PathBuf) -> Result<UserId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_uid)
}

/// Calls stat(2) on the file to determine an owner-group
pub fn get_file_gid(path: &PathBuf) -> Result<GroupId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

/// Calls stat(2) on the file to determine rights on the file
pub fn get_file_mode(path: &PathBuf) -> Result<FileMode> {
    let stat: libc::stat = unsafe { stat_file(path)? };
    Ok(stat.st_mode)
}

/// Wraps result of stat(2) call
unsafe fn stat_file(path: &PathBuf) -> Result<stat> {
    let path = ::native_path(path)?;
    let mut buf: stat = std::mem::zeroed();
    let status: c_int = stat(path.into_raw() as *const c_char, &mut buf);
    errno!(status, buf)
}
