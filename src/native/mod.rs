extern crate libc;

use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::exit;

use self::errno::Errno;

mod errno;
pub mod file_stat;
pub mod users;

use self::libc::{c_char, c_int, c_void, getcwd, gethostname, open, read, ssize_t, strlen, write};

/// Gets the name of the host using gethostname() from libc.
/// Returns None in case of error in gethostname() or in String::from_utf8().
pub fn get_hostname() -> Result<String> {
    let mut buf = vec![0; 256]; // MAXHOSTNAMELEN is unavailable in libc :(
    let result: c_int = unsafe { gethostname(buf.as_mut_ptr() as *mut c_char, buf.capacity()) };
    if result == 0 {
        unsafe {
            let len = strlen(buf.as_ptr() as *const c_char);
            buf.set_len(len);
        }
        read_buf(buf)
    } else {
        Err(Error::from_errno())
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

pub fn write_to_file(fd: RawFd, text: &str) -> Result<isize> {
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
        unsafe {
            let len = strlen(buf.as_ptr() as *const c_char);
            buf.set_len(len);
        }
        let dir = read_buf(buf)?;
        Ok(PathBuf::from(dir))
    }
}

pub fn read_file(fdi: RawFd) -> Result<String> {
    let mut result = Vec::new();
    let mut buf = vec![0; 256]; // because I can
    let mut status;
    loop {
        status = unsafe { read(fdi, buf.as_mut_ptr() as *mut c_void, buf.capacity()) };
        if status <= 0 {
            break;
        }
        let slice = &buf[0..status as usize];
        result.extend_from_slice(slice);

    }
    if status < 0 {
        Err(Error::Errno(Errno::last()))
    } else {
        read_buf(result)
    }
}

pub type ExitCode = i32;

pub fn write_exit(exit_code: ExitCode, text: &str) -> ! {
    write_to_file(2, text).ok();
    exit(exit_code);
}

/// Makes a copy of a string which was allocated by the system.
/// Otherwise Rust tries to manage the memory of the string which leads to segfault.
pub unsafe fn copy_string(ptr: *const c_char) -> Result<String> {
    let len = strlen(ptr);
    let mut buf = vec![0; len];
    libc::strcpy(buf.as_mut_ptr() as *mut c_char, ptr);
    read_buf(buf)
}

fn read_buf(buf: Vec<u8>) -> Result<String> {
    String::from_utf8(buf).map_err(|_| Error::InvalidUnicode)
}

fn native_string(string: &str) -> Result<CString> {
    CString::new(string).map_err(|_| Error::InvalidCString)
}

/// Creates a null terminated string out of an PathBuf instance
pub fn native_path(path: &PathBuf) -> Result<CString> {
    let path = path.to_str().ok_or(Error::InvalidUnicode)?;
    native_string(path)
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidCString,
    InvalidUnicode,
    NotFound,
    Errno(Errno),
}

impl Error {
    fn from_errno() -> Self {
        ::Error::Errno(Errno::last())
    }
}
