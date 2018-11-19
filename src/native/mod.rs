extern crate libc;

use std::ffi::CString;
use std::fmt::Display;
use std::fmt::Formatter;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::exit;

use self::libc::{__errno_location, c_char, c_int, c_void, getcwd, gethostname, getpwuid, open, passwd, read,
                 ssize_t, stat, strerror, strlen, write};

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Result<String> {
    let mut buf = vec![0; 256]; // MAXHOSTNAMELEN is unavailable in libc :(
    let result: c_int = unsafe { gethostname(buf.as_mut_ptr() as *mut c_char, buf.capacity()) };
    if result == 0 {
        read_buf(buf)
    } else {
        Err(Error::from_errno())
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
pub fn get_home_dir(uid: UserId) -> Result<PathBuf> {
    let entry: *const passwd = unsafe { getpwuid(uid) };
    if entry.is_null() {
        Err(Error::from_errno())
    } else {
        let dir: *const c_char = unsafe { (*entry).pw_dir };
        if dir.is_null() {
            Err(Error::NotFound)
        } else {
            let path = unsafe { copy_string(dir)? };
            Ok(PathBuf::from(path))
        }
    }
}

pub fn open_file(path: &PathBuf, flags: i32) -> Result<RawFd> {
    let path = native_path(path)?;
    let status: c_int = unsafe { open(path.into_raw() as *const c_char, flags) };
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        Ok(status)
    }
}

pub fn get_file_uid(path: &PathBuf) -> Result<UserId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_uid)
}

pub fn get_file_gid(path: &PathBuf) -> Result<GroupId> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

pub fn get_file_mode(path: &PathBuf) -> Result<FileMode> {
    let stat: libc::stat = unsafe { stat_file(path)? };
    Ok(stat.st_mode)
}

pub fn write_to_file(fd: RawFd, text: String) -> Result<isize> {
    let len = text.len();
    let text = native_string(text)?;
    let status: ssize_t = unsafe { write(fd, text.into_raw() as *const c_void, len) };
    if status < 0 {
        Err(Error::from_errno())
    } else {
        Ok(status)
    }
}

pub fn get_current_dir() -> Result<PathBuf> {
    let mut buf = vec![0; libc::PATH_MAX as usize];
    let name_ptr = unsafe { getcwd(buf.as_mut_ptr() as *mut c_char, buf.capacity()) };
    if name_ptr.is_null() {
        Err(Error::Errno(Errno::last()))
    } else {
        let dir = read_buf(buf)?;
        Ok(PathBuf::from(dir))
    }
}

pub fn read_file(fdi: RawFd) -> Result<String> {
    let mut result = Vec::new();
    let mut buf = vec![0; 256]; // because I can
    let mut status = unsafe { libc::read(fdi, buf.as_mut_ptr() as *mut c_void, buf.capacity()) };
    while status > 0 {
        let slice = &buf[0..status as usize];
        result.extend_from_slice(slice);
        status = unsafe { libc::read(fdi, buf.as_mut_ptr() as *mut c_void, buf.capacity()) };
    }
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        read_buf(result)
    }
}

pub type ExitCode = i32;

pub fn write_exit(exit_code: ExitCode, text: String) -> ! {
    write_to_file(2, text).ok();
    exit(exit_code);
}

unsafe fn stat_file(path: &PathBuf) -> Result<stat> {
    let path = native_path(path)?;
    let mut buf: stat = std::mem::zeroed();
    let status: c_int = stat(path.into_raw() as *const c_char, &mut buf);
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        Ok(buf)
    }
}

/// Makes a copy of a string which was allocated by the system.
/// Otherwise Rust tries to manage the memory of the string which leads to segfault.
unsafe fn copy_string(ptr: *const c_char) -> Result<String> {
    let len = strlen(ptr);
    let mut buf = vec![0; len];
    libc::strcpy(buf.as_mut_ptr() as *mut c_char, ptr);
    read_buf(buf)
}

fn read_buf(buf: Vec<u8>) -> Result<String> {
    String::from_utf8(buf).map_err(|_| Error::InvalidUnicode)
}

fn native_string(string: String) -> Result<CString> {
    CString::new(string).map_err(|_| Error::InvalidCString)
}

/// Creates a null terminated string out of an PathBuf instance
fn native_path(path: &PathBuf) -> Result<CString> {
    let path = path.to_str().ok_or(Error::InvalidUnicode)?;
    let path = String::from(path);
    native_string(path)
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidCString,
    InvalidUnicode,
    NotFound,
    Errno(Errno)
}

impl Error {
    fn from_errno() -> Self {
        ::Error::Errno(Errno::last())
    }
}

#[derive(Debug)]
pub struct Errno {
    code: c_int,
    text: String,
}

impl Errno {
    pub fn last() -> Self {
        let errno_ptr: *const c_int = unsafe { __errno_location() };
        if errno_ptr.is_null() {
            write_exit(1, String::from("errno location is unknown"));
        } else {
            let code: c_int = unsafe { *errno_ptr };
            let text: *const c_char = unsafe { strerror(code) };
            if text.is_null() {
                write_exit(2, String::from("errno code is unknown"));
            } else {
                if let Ok(text) = unsafe { copy_string(text) } {
                    Errno { code, text }
                } else {
                    write_exit(3, String::from("errno string is incorrect C string"));
                }
            }
        }
    }
}
