extern crate libc;

use std::path::PathBuf;

use self::libc::{stat, c_int, c_char};

use {Result, Error};

pub fn get_file_uid(path: &PathBuf) -> Result<::UserId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_uid)
}

pub fn get_file_gid(path: &PathBuf) -> Result<::GroupId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

pub fn get_file_mode(path: &PathBuf) -> Result<FileMode> {
    let stat: libc::stat = unsafe { stat_file(path)? };
    Ok(stat.st_mode)
}

unsafe fn stat_file(path: &PathBuf) -> Result<stat> {
    let path = ::native_path(path)?;
    let mut buf: stat = std::mem::zeroed();
    let status: c_int = stat(path.into_raw() as *const c_char, &mut buf);
    errno!(status, buf)
}
