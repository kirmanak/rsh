extern crate libc;

use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::exit;

use self::libc::{__errno_location, c_char, c_int, c_void, getcwd, gethostname, getpwuid, open, passwd, read,
                 ssize_t, stat, strerror, strlen, write};

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Result<CString, Error> {
    let mut buf = vec![0; 256]; // MAXHOSTNAMELEN is unavailable in libc :(
    let result: c_int = unsafe { gethostname(buf.as_mut_ptr() as *mut c_char, buf.capacity()) };
    if result == 0 {
        let string = unsafe { read_buf(buf)? };
        Ok(string)
    } else {
        Err(Error::NotFound)
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
pub fn get_home_dir(uid: UserId) -> Result<CString, Error> {
    let entry: *const passwd = unsafe { getpwuid(uid) };
    if entry.is_null() {
        Err(Error::NotFound)
    } else {
        let dir: *const c_char = unsafe { (*entry).pw_dir };
        if dir.is_null() {
            Err(Error::NotFound)
        } else {
            unsafe { copy_string(dir) }
        }
    }
}

pub fn open_file(path: &PathBuf, flags: i32) -> Result<RawFd, Error> {
    let path = native_path(path)?;
    let status: c_int = unsafe { open(path.into_raw() as *const c_char, flags) };
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        Ok(status)
    }
}

pub fn get_file_uid(path: &PathBuf) -> Result<UserId, Error> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_uid)
}

pub fn get_file_gid(path: &PathBuf) -> Result<GroupId, Error> {
    let stat = unsafe { stat_file(path)? };
    Ok(stat.st_gid)
}

pub type FileMode = u32;

pub fn get_file_mode(path: &PathBuf) -> Result<FileMode, Error> {
    let stat: libc::stat = unsafe { stat_file(path)? };
    Ok(stat.st_mode)
}

pub fn write_to_file(fd: RawFd, text: &CString) -> Result<isize, Error> {
    let len = text.as_bytes_with_nul().len();
    let status: ssize_t = unsafe { write(fd, text.into_raw() as *const c_void, len) };
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        Ok(status)
    }
}

pub fn get_current_dir() -> Result<CString, Error> {
    let mut buf = vec![0; libc::PATH_MAX as usize];
    let name_ptr = unsafe { getcwd(buf.as_mut_ptr() as *mut c_char, buf.capacity()) };
    if name_ptr.is_null() {
        Err(Error::Errno(Errno::last()))
    } else {
        read_buf(buf)
    }
}

pub fn read_file(fdi: RawFd) -> Result<CString, Error> {
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

pub fn write_exit(exit_code: ExitCode, text: &CString) -> ! {
    write_to_file(2, text).ok();
    unsafe { libc::exit(exit_code) }
}

pub fn os_to_c(os: OsString) -> Result<CString, Error> {
    CString::new(os).map_err(|_| Error::InvalidCString)
}

unsafe fn stat_file(path: &PathBuf) -> Result<stat, Error> {
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
unsafe fn copy_string(ptr: *const c_char) -> Result<CString, Error> {
    let len = strlen(ptr);
    let mut buf = vec![0; len];
    libc::strcpy(buf.as_mut_ptr() as *mut c_char, ptr);
    read_buf(buf)
}

fn read_buf(buf: Vec<u8>) -> Result<CString, Error> {
    CString::new(buf).map_err(|_| Error::InvalidCString)
}

fn native_path(path: &PathBuf) -> Result<CString, Error> {
    os_to_c(path.into_os_string())
}

pub enum Error {
    InvalidCString,
    NotFound,
    Errno(Errno),
}

pub struct Errno {
    code: c_int,
    text: CString,
}

impl Errno {
    pub fn last() -> Errno {
        let errno_ptr: *const c_int = unsafe { __errno_location() };
        if errno_ptr.is_null() {
            if let Ok(text) = CString::new("errno location is unknown") {
                write_exit(1, &text);
            } else {
                exit(1);
            }
        } else {
            let code: c_int = unsafe { *errno_ptr };
            let text: *const c_char = unsafe { strerror(code) };
            if text.is_null() {
                if let Ok(text) = CString::new("errno code is unknown") {
                    write_exit(2, &text);
                } else {
                    exit(2);
                }
            } else {
                if let Ok(text) = unsafe { copy_string(text) } {
                    Errno { code, text }
                } else {
                    if let Ok(text) = CString::new("errno string is incorrect C string") {
                        write_exit(3, &text);
                    } else {
                        exit(3);
                    }
                }
            }
        }
    }
}
