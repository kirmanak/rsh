use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Option<String> {
    let capacity = 256; // according to man, any host name is limited to 256 characters
    let buf = Vec::with_capacity(capacity);
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
pub fn get_home_dir(uid: UserId) -> Option<String> {
    let pwd_entry: *mut libc::passwd = unsafe { libc::getpwuid(uid) };
    if pwd_entry.is_null() {
        None
    } else {
        let dir = unsafe { (*pwd_entry).pw_dir };
        if dir.is_null() {
            None
        } else {
            let dir = unsafe { CString::from_raw(dir) };
            dir.into_string().ok()
        }
    }
}

pub type FileDescriptorId = i32;

pub fn open_file(path: &str, flags: i32) -> Result<FileDescriptorId> {
    let path = CString::new(path)?;
    let status = unsafe { libc::open(path.into_raw(), flags) };
    if status < 0 {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        Ok(status)
    }
}

pub fn get_file_uid(path: &str) -> Result<UserId> {
    let path = CString::new(path)?;
    let stat = unsafe { stat_file(&path)? };
    Ok(stat.st_uid)
}

pub fn get_file_gid(path: &str) -> Result<GroupId> {
    let path = CString::new(path)?;
    let stat = unsafe { stat_file(&path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

pub fn get_file_mode(path: &str) -> Result<FileMode> {
    let path = CString::new(path)?;
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

pub fn get_current_dir() -> Result<String> {
    let capacity = libc::PATH_MAX as usize;
    let mut buf = Vec::with_capacity(capacity);
    let buf = unsafe { CString::from_vec_unchecked(buf) };
    let buf = buf.into_raw();
    let name_ptr = unsafe { libc::getcwd(buf, capacity) };
    if name_ptr.is_null() {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        let current_dir = unsafe { CString::from_raw(buf) };
        current_dir.into_string()
            .map_err(|reason| Error::new(ErrorKind::InvalidData, reason))
    }
}

pub fn read_file(fdi: FileDescriptorId) -> Result<String> {
    let capacity = 256; // because I can
    let buf = Vec::with_capacity(capacity);
    let buf = unsafe { CString::from_vec_unchecked(buf) };
    let buf_ptr = buf.into_raw();
    let mut result = String::new();
    let mut status = unsafe { libc::read(fdi, buf_ptr as *mut libc::c_void, capacity) };
    while status > 0 {
        let data = unsafe { CString::from_raw(buf_ptr) };
        let data = data.to_str()
            .map_err(|reason| Error::new(ErrorKind::InvalidData, reason))?;
        result.push_str(data);
        status = unsafe { libc::read(fdi, buf_ptr as *mut libc::c_void, capacity) };
    }
    if status < 0 {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        Ok(result)
    }
}

pub type ExitCode = i32;

pub fn exit_error(exit_code: ExitCode, text: &str) -> ! {
    write_to_file(2, text);
    unsafe { libc::exit(exit_code) }
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
    let len = libc::strlen(text_ptr);
    // MUST NOT be CString, otherwise Rust tries to free the memory, but it causes a segfault
    let error_text = String::from_raw_parts(text_ptr as *mut u8, len, len);
    let kind = match errno {
        1 => ErrorKind::PermissionDenied,
        2 => ErrorKind::NotFound,
        4 => ErrorKind::Interrupted,
        _ => ErrorKind::Other,
    };
    Error::new(kind, error_text)
}
