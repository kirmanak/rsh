use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Option<String> {
    let capacity = 256; // MAXHOSTNAMELEN is unavailable in libc :(
    let buf = unsafe { create_buf(capacity) };
    let status = unsafe { libc::gethostname(buf, capacity) };
    if status == 0 {
        let raw_string = unsafe { CString::from_raw(buf) };
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
pub fn get_home_dir(uid: UserId) -> Result<String> {
    let pwd_entry: *mut libc::passwd = unsafe { libc::getpwuid(uid) };
    if pwd_entry.is_null() {
        let error = unsafe { get_errno() };
        Err(error)
    } else {
        let dir = unsafe { (*pwd_entry).pw_dir };
        if dir.is_null() {
            Err(Error::new(ErrorKind::NotFound, "Home directory is not found"))
        } else {
            let dir = unsafe { copy_raw(dir) };
            dir.into_string().map_err(|reason| Error::new(
                ErrorKind::InvalidData,
                reason,
            ))
        }
    }
}

pub type FileDescriptorId = i32;

pub fn open_file(path: &str, flags: i32) -> Result<FileDescriptorId> {
    let path = CString::new(path)?.into_raw();
    let status = unsafe { libc::open(path, flags) };
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
    let buf = unsafe { create_buf(capacity) };
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
    let buf = unsafe { create_buf(capacity) };
    let mut result = String::new();
    let mut status = unsafe { libc::read(fdi, buf as *mut libc::c_void, capacity) };
    while status > 0 {
        let data = unsafe { CString::from_raw(buf) };
        let data = data.to_str()
            .map_err(|reason| Error::new(ErrorKind::InvalidData, reason))?;
        result.push_str(data);
        status = unsafe { libc::read(fdi, buf as *mut libc::c_void, capacity) };
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
    write_to_file(2, text).unwrap();
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

/// Makes a copy of a string which was allocated by the system.
/// Otherwise Rust tries to manage the memory of the string which leads to segfault.
unsafe fn copy_raw(ptr: *mut libc::c_char) -> CString {
    let buf = create_buf(libc::strlen(ptr));
    libc::strcpy(buf, ptr);
    CString::from_raw(buf)
}

unsafe fn get_errno() -> Error {
    let errno_ptr = libc::__errno_location();
    let text_ptr = if errno_ptr.is_null() {
        libc::PT_NULL as *mut libc::c_char
    } else {
        libc::strerror(*errno_ptr) // can return null as well
    };
    let error_text: String = if text_ptr.is_null() {
        "The description of the error is unavailable.".to_string()
    } else {
        copy_raw(text_ptr).into_string()
            .unwrap_or("The description of the error is not valid UTF-8 string!".to_string())
    };
    let kind = if errno_ptr.is_null() {
        ErrorKind::Other
    } else {
        match *errno_ptr {
            1 => ErrorKind::PermissionDenied,
            2 => ErrorKind::NotFound,
            4 => ErrorKind::Interrupted,
            _ => ErrorKind::Other,
        }
    };
    Error::new(kind, error_text)
}

unsafe fn create_buf(capacity: usize) -> *mut libc::c_char {
    CString::from_vec_unchecked(
        vec![0; capacity]
    ).into_raw()
}
