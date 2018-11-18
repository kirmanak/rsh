use std::collections::HashMap;
use std::env::args;
use std::env::var;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;
use std::ops::Add;

use splitter::split_arguments;
use syscalls::{GroupId, UserId};

mod splitter;
mod syscalls;

fn main() {
    let shell = Shell::new().unwrap();
    if shell.is_login {
        shell.interpret("/etc/.login");
        shell.interpret_rc(".cshrc");
        shell.interpret_rc(".login");
    } else {
        shell.interpret_rc(".cshrc");
    }
    if shell.argv.len() > 1 {
        shell.argv.iter() // iterating over argv
            .skip(1) // skipping the name of the shell
            .filter(|arg| !arg.starts_with('-')) // filtering options
            .for_each(|path| {
                if let Err(reason) = shell.interpret(path) {
                    let error = format!("{}: {}", path, &reason);
                    syscalls::exit_error(1, &error);
                }
            });
    } else {
        shell.interact();
    }
}


/// Checks whether the file is readable and either is owned by the current user
/// or the current user's real group ID matches the file's group ID
fn check_file(path: &str) -> Result<bool> {
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
    pub argv: Vec<String>,
    pub user: syscalls::UserId,
    pub status: syscalls::ExitCode,
}

impl Shell {
    pub fn new() -> Result<Shell> {
        let user = syscalls::get_uid();
        let mut variables: HashMap<String, String> = HashMap::new();
        variables.insert(String::from("path"), var("PATH").unwrap_or(String::from("/usr/bin")));
        variables.insert(String::from("home"), syscalls::get_home_dir(user)?);
        variables.insert(String::from("cwd"), syscalls::get_current_dir()?);
        variables.insert(String::from("prompt"), Self::get_prompt(user));
        let mut argv = args().collect();
        Ok(Shell {
            variables,
            is_login: Self::is_login(&argv),
            argv,
            user,
            status: 0,
        })
    }

    pub fn interpret(&self, path: &str) -> Result<()> {
        let fdi = syscalls::open_file(path, libc::O_RDONLY)?;
        let content = syscalls::read_file(fdi)?;
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
            syscalls::write_to_file(1, &arg);
        }
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
        match &self.variables.get("home") {
            None => return Err(Error::new(ErrorKind::NotFound, "home dir is not found")),
            Some(home) => {
                let mut rc_file = std::path::PathBuf::from(home);
                rc_file.push(rc_name);
                let rc_file = rc_file.to_str()
                    .ok_or(Error::new(ErrorKind::InvalidData, "Path is not valid UTF-8"))?;
                return if check_file(&rc_file)? {
                    self.interpret(&rc_file)
                } else {
                    Ok(())
                };
            }
        }
    }

    pub fn interact(&self) -> Result<()> {
        println!("Hello, world!");
        let prompt = self.get_variable("prompt")?;
        syscalls::write_to_file(1, prompt)?;
        let input = syscalls::read_file(0)?;
        if input.contains("pwd") {
            let cwd = self.get_variable("cwd")?;
            syscalls::write_to_file(1, cwd)?;
        }
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Result<&String> {
        let error_text = format!("{} variable is not found", name);
        let error_text = error_text.as_str();
        self.variables.get(name)
            .ok_or(Error::new(ErrorKind::NotFound, error_text))
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
