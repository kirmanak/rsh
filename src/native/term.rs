extern crate libc;

use self::libc::{termios, tcgetattr, tcsetattr, c_int};

use std::os::unix::io::RawFd;

use {Result, Error};

pub fn setup_tty(fd: RawFd, on: i32) -> Result<()> {
    let configuration = unsafe { get_attr(fd)? };

    Ok(())
}

unsafe fn get_attr(fd: RawFd) -> Result<termios> {
    let mut buf: termios = std::mem::zeroed();
    let result: c_int = tcgetattr(fd, &mut buf);
    errno!(result, buf)
}
