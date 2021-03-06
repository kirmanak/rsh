pub mod error;

use self::error::*;

#[macro_export]
macro_rules! errno {
    ($status:expr, $result:expr) => {
        if $status < 0 {
            Err(Error::from_errno())
        } else {
            Ok($result)
        }
    };
}

macro_rules! unwrap_or_return {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(reason) => return reason,
        }
    };
}

use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::process::exit;
use std::ptr::null;
use std::iter::once;

pub mod file_stat;
pub mod users;

use libc::{c_char, c_int, c_void, getcwd, gethostname, open, read, ssize_t, strlen, write, execve,
           fork, waitpid, dup2, PATH_MAX, strcpy};

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

/// Opens the file which is located on the provided path with the provided flags.
/// More information about the flags is in open(2).
/// These constants are available in libc crate.
pub fn open_file(path: &PathBuf, flags: i32, mode: Option<u32>) -> Result<RawFd> {
    let path = native_path(path)?;
    let status: c_int = match mode {
        Some(mode) => unsafe { open(path.into_raw() as *const c_char, flags, mode) },
        None => unsafe { open(path.into_raw() as *const c_char, flags) },
    };
    errno!(status, status)
}

//// Writes text to the file and returns non-negative number in the case of success.
pub fn write_to_file(fd: RawFd, text: &str) -> Result<isize> {
    let len = text.len();
    let text = native_string(text)?;
    let status: ssize_t = unsafe { write(fd, text.into_raw() as *const c_void, len) };
    errno!(status, status)
}

/// Gets current working dir from the system
pub fn get_current_dir() -> Result<PathBuf> {
    let mut buf = vec![0; PATH_MAX as usize];
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

/// Reads file contents to a String
pub fn read_file(fdi: RawFd) -> Result<String> {
    let mut result = Vec::new();
    let mut buf = vec![0; 4096]; // like in csh
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

/// Reads a line (chars till '\n' or EOF) from the provided file
pub fn read_line(fdi: RawFd) -> Result<String> {
    let mut result = Vec::new();
    let mut buf = [0; 1];
    let mut status;
    loop {
        status = unsafe { read(fdi, buf.as_mut_ptr() as *mut c_void, 1) };
        let c = buf[0];
        if status <= 0 || c == b'\n' {
            break;
        }
        result.push(c);
    }
    if status < 0 {
        Err(Error::from_errno())
    } else {
        read_buf(result)
    }
}

pub type ExitCode = i32;

/// Writes the provided text to stderr and exits with the provided exit code.
pub fn write_exit(exit_code: ExitCode, text: &str) -> ! {
    write_to_file(2, text).ok();
    exit(exit_code);
}

/// Makes a copy of a string which was allocated by the system.
/// Otherwise Rust tries to manage the memory of the string which leads to segfault.
pub unsafe fn copy_string(ptr: *const c_char) -> Result<String> {
    let len = strlen(ptr);
    let mut buf = vec![0; len];
    strcpy(buf.as_mut_ptr() as *mut c_char, ptr);
    read_buf(buf)
}

pub fn replace_fdi(to_replace: RawFd, replacement: RawFd) -> Result<()> {
    let status = unsafe { dup2(replacement, to_replace) };
    errno!(status, ())
}

/// Wraps Vec<u8> to String
fn read_buf(buf: Vec<u8>) -> Result<String> {
    String::from_utf8(buf).map_err(|_| Error::InvalidUnicode)
}

/// Wraps string slice to CString
fn native_string(string: &str) -> Result<CString> {
    CString::new(string).map_err(|_| Error::InvalidCString)
}

/// Creates a null terminated string out of an PathBuf instance
pub fn native_path(path: &PathBuf) -> Result<CString> {
    let path = path.to_str().ok_or(Error::InvalidUnicode)?;
    native_string(path)
}

/// Forks the current process and calls the provided function
pub fn fork_process<F: FnOnce() -> Error>(actions: F) -> Result<i32> {
    match unsafe { fork() } {
        0 => Err(actions()), // if we returned from actions, something went wrong
        -1 => Err(Error::from_errno()),
        _ => {
            let mut status = 0;
            unsafe {
                waitpid(-1, &mut status, 0);
            }
            Ok(status)
        }
    }
}

/// Creates pointers to arguments readable by C and executes the program
pub fn execute(path: &PathBuf, args: Vec<String>, envp: Vec<String>) -> Error {
    let path = unwrap_or_return!(native_path(path));
    // MUST NOT be shadowed otherwise will be freed
    let mut native_args = Vec::with_capacity(args.len());
    for arg in args {
        let native = unwrap_or_return!(native_string(&arg));
        native_args.push(native);
    }
    let args: Vec<*const i8> = native_args
        .iter()
        .map(|s| s.as_ptr())
        .chain(once(null()))
        .collect();
    // MUST NOT be shadowed otherwise will be freed
    let mut native_envp = Vec::with_capacity(envp.len());
    for arg in envp {
        let native = unwrap_or_return!(native_string(&arg));
        native_envp.push(native);
    }
    let envp: Vec<*const i8> = native_envp
        .iter()
        .map(|s| s.as_ptr())
        .chain(once(null()))
        .collect();
    unsafe {
        execve(path.as_ptr(), args.as_ptr(), envp.as_ptr());
    }
    Error::from_errno()
}
