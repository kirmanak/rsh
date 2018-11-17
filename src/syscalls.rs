extern crate libc;

use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Option<String> {
    let capacity = 256; // according to man, any host name is limited to 256 characters
    let mut buf = Vec::with_capacity(capacity);
    let raw_string = unsafe { CString::from_vec_unchecked(buf) };
    let raw_string = raw_string.into_raw();
    let status = unsafe { libc::gethostname(raw_string, capacity) };
    if status == 0 {
        let raw_string = unsafe { CString::from_raw(raw_string) };
        raw_string.into_string().ok()
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
        let dir = unsafe { (*pwd_entry).pw_dir };
        if dir.is_null() {
            None
        } else {
            let dir = unsafe { CString::from_raw(dir) };
            let dir = dir.to_str().ok()?;
            Some(PathBuf::from(dir))
        }
    }
}

pub type FileDescriptorId = i32;

pub fn open_file(path: &PathBuf, flags: i32) -> Result<FileDescriptorId> {
    let path = path.to_str()
        .ok_or(Error::new(ErrorKind::InvalidData, "Invalid file path"))?;
    let status = unsafe { libc::open(path.as_ptr() as *const i8, flags) };
    if status < 0 {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        Ok(status)
    }
}

pub fn get_file_uid(path: &PathBuf) -> Result<UserId> {
    let path = path_to_str(path)?;
    let stat: libc::stat = unsafe { stat_file(&path)? };
    Ok(stat.st_uid)
}

pub fn get_file_gid(path: &PathBuf) -> Result<GroupId> {
    let path = path_to_str(path)?;
    let stat: libc::stat = unsafe { stat_file(&path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

pub fn get_file_mode(path: &PathBuf) -> Result<FileMode> {
    let path = path_to_str(path)?;
    let stat: libc::stat = unsafe { stat_file(&path)? };
    Ok(stat.st_mode)
}

pub fn write_to_file(fdi: FileDescriptorId, text: &str) -> Result<()> {
    let len = text.len();
    let text = CString::new(text)?;
    let status = unsafe { libc::write(fdi, text.into_raw() as *const libc::c_void, len) };
    if status < 0 {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        Ok(())
    }
}

pub type ExitCode = i32;

pub fn exit_error(exit_code: ExitCode, text: &str) -> ! {
    write_to_file(2, text);
    unsafe { libc::exit(exit_code) }
}

fn path_to_str(buf: &PathBuf) -> Result<CString> {
    let buf = buf.to_str()
        .ok_or(Error::new(ErrorKind::InvalidData, "Invalid file path"))?;
    let string = CString::new(buf)?;
    Ok(string)
}

unsafe fn stat_file(path: &CString) -> Result<libc::stat> {
    let mut buf: libc::stat = std::mem::zeroed();
    let status = libc::stat(path.as_ptr(), &mut buf);
    if status < 0 {
        Err(get_errno())
    } else {
        Ok(buf)
    }
}

unsafe fn get_errno() -> Error {
    let errno_ptr = libc::__errno_location();
    if errno_ptr.is_null() {
        exit_error(1, "errno location is unknown!");
    }
    let errno = *errno_ptr;
    let text_ptr = libc::strerror(errno);
    if text_ptr.is_null() {
        exit_error(1, "errno status is invalid!");
    }
    let error_text = CString::from_raw(text_ptr);
    let transform_result = error_text.to_str();
    if let Err(reason) = transform_result {
        let error_text = format!("errno text is not valid UTF-8: {}", reason);
        exit_error(1, error_text.as_str());
    }
    let error_text = transform_result.unwrap();
    let kind = match errno {
        1 => ErrorKind::PermissionDenied,
        2 => ErrorKind::NotFound,
        4 => ErrorKind::Interrupted,
        _ => ErrorKind::Other,
    };
    Error::new(kind, error_text)
}
