use std::collections::HashMap;
use std::env::args;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::ops::Add;
use std::path::PathBuf;

use splitter::split_arguments;
use syscalls::{GroupId, UserId};

mod splitter;
mod syscalls;

fn main() {
    let shell = Shell::new();
    if shell.is_login {
        shell.interpret(&(PathBuf::from("/etc/.login")));
        shell.interpret_rc(".cshrc");
        shell.interpret_rc(".login");
    } else {
        shell.interpret_rc(".cshrc");
    }
    let args: Vec<PathBuf> = args().skip(1) // skipping the name of the shell
        .filter(|arg| !arg.starts_with('-')) // filtering options
        .map(PathBuf::from)
        .collect();
    if args.len() > 0 {
        for argument in args {
            if let Err(reason) = shell.interpret(&argument) {
                panic!("{}: {}", argument.to_str().unwrap(), reason);
            }
        }
    }
}


/// Checks whether the file is readable and either is owned by the current user
/// or the current user's real group ID matches the file's group ID
fn check_file(path: &PathBuf) -> Result<bool> {
    let file_uid: UserId = syscalls::get_file_uid(&path)?;
    let file_gid: GroupId = syscalls::get_file_gid(&path)?;
    let user_uid: UserId = syscalls::get_uid();
    let user_gid: GroupId = syscalls::get_gid();
    let mode = syscalls::get_file_mode(&path)?;
    let can_user_read = mode & 0o400 != 0;
    let can_group_read = mode & 0o040 != 0;
    Ok((user_uid == file_uid && can_user_read) || (user_gid == file_gid && can_group_read))
}

pub struct Shell {
    pub variables: HashMap<String, String>,
    pub is_login: bool,
    pub home: Option<PathBuf>,
    pub path: Vec<PathBuf>,
    pub argv: Vec<String>,
    pub user: syscalls::UserId,
    pub cwd: Result<PathBuf>,
    pub prompt: String,
    pub status: Option<std::process::ExitStatus>,
}

impl Shell {
    pub fn new() -> Shell {
        let path = std::env::var("PATH").unwrap_or(String::from("/usr/bin"));
        let path = path.split(':').map(PathBuf::from).collect();
        let user = syscalls::get_uid();
        let argv = std::env::args().collect();
        Shell {
            variables: HashMap::new(),
            is_login: Self::is_login(&argv),
            home: syscalls::get_home_dir(user),
            path,
            cwd: std::env::current_dir(),
            prompt: Self::get_prompt(user),
            argv,
            user,
            status: None,
        }
    }

    pub fn interpret(&self, path: &PathBuf) -> Result<()> {
        let fdi = syscalls::open_file(path, libc::O_RDONLY)?;
        Ok(())
    }

    /// Parses the command and executes it
    fn execute(&self, line: &str) {
        let arguments = split_arguments(line);
    }

    /// Checks whether we're the login shell or not
    fn is_login(args: &Vec<String>) -> bool {
        match args.len() {
            0 => panic!("Something went REALLY wrong"), // first argument MUST be present
            1 => args[0].starts_with('-'), // we had no arguments and started as -<something>,
            2 => args[1].eq(&"-l".to_string()), // we had only one argument - "-l",
            _ => false
        }
    }

    /// Gets text for prompt from the system
    fn get_prompt(user: UserId) -> String {
        let hostname = syscalls::get_hostname().unwrap_or("hostname".to_string());
        let suffix = if user == 0 { "#" } else { "%" };
        hostname.add(suffix)
    }

    /// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
    pub fn interpret_rc(&self, rc_name: &str) -> Result<()> {
        match &self.home {
            None => return Err(Error::new(ErrorKind::NotFound, "home dir is not found")),
            Some(home) => {
                let mut rc_file = home.clone();
                rc_file.push(rc_name);
                return if check_file(&rc_file)? {
                    self.interpret(&rc_file)
                } else {
                    Ok(())
                };
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_login_regular() {
        let args: Vec<String> = vec!["rsh", "hello.rsh"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), false);
    }

    #[test]
    fn is_login_minus_and_arg() {
        let args = vec!["-rsh", "hello.rsh"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), false);
    }

    #[test]
    fn is_login_minus_no_args() {
        let args = vec!["-rsh"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), true);
    }

    #[test]
    fn is_login_argument_login() {
        let args = vec!["rsh", "-l"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), true);
    }

    #[test]
    fn is_login_argument_login_and_another() {
        let args = vec!["rsh", "-l", "hello.rsh"]
            .iter().map(|s| s.to_string()).collect();
        assert_eq!(Shell::is_login(&args), false);
    }
}
