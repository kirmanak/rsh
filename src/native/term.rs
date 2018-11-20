extern crate libc;

use self::libc::{termios, tcgetattr, tcsetattr, c_int};

use std::os::unix::io::RawFd;

use {Result, Error};

pub fn setup_tty(fd: RawFd, is_on: bool) -> Result<()> {
    let configuration = unsafe { get_attr(fd)? };

    Ok(())
}

/// Gets the current state of termios attributes on the provided file
unsafe fn get_attr(fd: RawFd) -> Result<termios> {
    let mut buf: termios = std::mem::zeroed();
    let result: c_int = tcgetattr(fd, &mut buf);
    errno!(result, buf)
}
