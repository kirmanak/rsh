extern crate nix;

use std::collections::HashMap;
use std::env::{args, var};
use std::ops::Add;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::ExitStatus;

use nix::fcntl::{OFlag, open};
use nix::Result;
use nix::sys::stat::{Mode, stat};
use nix::unistd::{getcwd, gethostname, Gid, read, Uid, write};

use native::get_home_dir;
use splitter::split_arguments;

mod splitter;
mod native;

/// This PATH is used when environmental variable PATH is not set
const DEFAULT_PATH: PathBuf = PathBuf::from("/usr/bin");
/// This path is used to find the first script which should be interpreted by the login shell
const FIRST_LOGIN: PathBuf = PathBuf::from("/etc/.login");
/// This path is used to find the second script which should be interpreted by the login shell
const SECOND_LOGIN: PathBuf = PathBuf::from(".cshrc");
/// This path is used to find the third script which should be interpreted by the login shell
const THIRD_LOGIN: PathBuf = PathBuf::from(".login");
/// This path is used to find the script which should be interpreted  by a non-login shell
const NO_LOGIN: PathBuf = SECOND_LOGIN;

fn main() {
    let shell = Shell::new().unwrap();
    if shell.is_login {
        shell.interpret(&FIRST_LOGIN).ok();
        shell.interpret_rc(&SECOND_LOGIN).ok();
        shell.interpret_rc(&THIRD_LOGIN).ok();
    } else {
        shell.interpret_rc(&NO_LOGIN).ok();
    }
    if shell.argv.len() > 1 {
        shell.argv.iter() // iterating over argv
            .skip(1) // skipping the name of the shell
            .filter(|arg| !arg.starts_with('-')) // filtering options
            .for_each(|path| {
                if let Err(reason) = shell.interpret(&PathBuf::from(path)) {
                    let error = format!("{}: {}", path.as_str(), reason.to_string().as_str());
                    write(1, error.as_bytes());
                }
            });
    } else {
        shell.interact().ok();
    }
}


/// Checks whether the file is readable and either is owned by the current user
/// or the current user's real group ID matches the file's group ID
fn check_file(path: &PathBuf) -> Result<bool> {
    let file_stat = stat(path)?;
    let file_uid = Uid::from_raw(file_stat.st_uid);
    let file_gid = Gid::from_raw(file_stat.st_gid);
    let user_uid = Uid::current();
    let user_gid = Gid::current();
    let mode = Mode::from_bits_truncate(file_stat.st_mode);
    Ok(
        (user_uid == file_uid && mode.contains(Mode::S_IRUSR)) || (user_gid == file_gid && mode.contains(Mode::S_IRGRP)),
    )
}

pub struct Shell {
    pub variables: HashMap<String, String>,
    pub is_login: bool,
    pub argv: Vec<String>,
    pub user: Uid,
    pub status: ExitStatus,
    pub cwd: PathBuf,
    pub path: Vec<PathBuf>,
    pub prompt: String,
    pub home: PathBuf,
}

impl Shell {
    pub fn new() -> Result<Shell> {
        let argv = args().collect();
        let user = Uid::current();
        Ok(Shell {
            cwd: getcwd()?,
            variables: HashMap::new(),
            is_login: Self::is_login(&argv),
            argv,
            user,
            status: ExitStatus::from_raw(0),
            path: var("PATH").unwrap_or(String::from("/usr/bin")).split(':').map(PathBuf::from).collect(),
            prompt: Self::get_prompt(user)?,
            home: PathBuf::from(get_home_dir(user)?),
        })
    }

    pub fn interpret(&self, path: &PathBuf) -> Result<()> {
        let fdi = open(path, OFlag::O_RDONLY, Mode::empty())?;
        let mut buf = [0; 256];
        let result = read(fdi, &mut buf)?;
        let buf = &buf[0..result]; // ignore unused space
        let content = String::from(String::from_utf8_lossy(&buf));
        for line in content.lines() {
            self.execute(line);
        }
        Ok(())
    }

    /// Parses the command and executes it
    fn execute(&self, line: &str) {
        let arguments = split_arguments(line);
        for arg in arguments {
            let arg = format!("{}\n", arg);
            write(1, arg.as_bytes()).ok();
        }
    }

    /// Checks whether we're the login shell or not
    fn is_login(args: &Vec<String>) -> bool {
        match args.len() {
            0 => panic!("Something went REALLY wrong"), // first argument MUST be present
            1 => args[0].starts_with('-'), // we had no arguments and started as -<something>,
            2 => args[1].eq(&"-l".to_string()), // we had only one argument - "-l",
            _ => false,
        }
    }

    /// Gets text for prompt from the system
    fn get_prompt(user: Uid) -> Result<String> {
        let mut buf = [0u8; 256];
        let hostname = gethostname(&mut buf)?;
        let hostname = String::from(hostname.to_string_lossy());
        let suffix = if user.is_root() { "# " } else { "% " };
        Ok(hostname.add(suffix))
    }

    /// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
    pub fn interpret_rc(&self, rc_name: &PathBuf) -> Result<()> {
        let mut rc_file = &self.home;
        rc_file.push(rc_name);
        return if check_file(&rc_file)? {
            self.interpret(&rc_file)
        } else {
            Ok(())
        };
    }

    pub fn interact(&self) -> Result<()> {
        let prompt = self.prompt.as_bytes();
        let mut buf = [0; 256];
        write(1, prompt)?;
        let result = read(0, &mut buf)?;
        let buf = &buf[0..result];
        let input = String::from(String::from_utf8_lossy(&buf));
        if input.contains("pwd") {
            let cwd = &self.cwd;
            let cwd = String::from(cwd.to_string_lossy()).as_bytes();
            write(1, cwd)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_login_regular() {
        let args: Vec<String> = vec!["rsh", "hello.rsh"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(Shell::is_login(&args), false);
    }

    #[test]
    fn is_login_minus_and_arg() {
        let args = vec!["-rsh", "hello.rsh"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(Shell::is_login(&args), false);
    }

    #[test]
    fn is_login_minus_no_args() {
        let args = vec!["-rsh"].iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), true);
    }

    #[test]
    fn is_login_argument_login() {
        let args = vec!["rsh", "-l"].iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), true);
    }

    #[test]
    fn is_login_argument_login_and_another() {
        let args = vec!["rsh", "-l", "hello.rsh"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(Shell::is_login(&args), false);
    }
}
