use std::collections::HashMap;
use std::env::{args, var};
use std::ops::Add;
use std::path::PathBuf;

use native::*;
use splitter::split_arguments;

mod splitter;
mod native;

fn main() {
    if let Ok(shell) = Shell::new() {
        if shell.is_login {
            shell.interpret(&PathBuf::from("/etc/.login")).ok();
            shell.interpret_rc(".cshrc").ok();
            shell.interpret_rc(".login").ok();
        } else {
            shell.interpret_rc(".cshrc").ok();
        }
        if shell.argv.len() > 1 {
            shell.argv.iter() // iterating over argv
                .skip(1) // skipping the name of the shell
                .filter(|arg| !arg.starts_with('-')) // filtering options
                .map(PathBuf::from)
                .for_each(|path| {
                    if let Err(reason) = shell.interpret(&path) {
                        let error = format!("{:?}: {:?}", path, reason);
                        write_exit(5, error);
                    }
                });
        } else {
            shell.interact().ok();
        }
    } else {
        write_exit(4,String::from("Failed to initialize the shell"));
    }
}


/// Checks whether the file is readable and either is owned by the current user
/// or the current user's real group ID matches the file's group ID
fn check_file(path: &PathBuf) -> Result<bool> {
    let file_uid: UserId = get_file_uid(&path)?;
    let file_gid: GroupId = get_file_gid(&path)?;
    let user_uid: UserId = get_uid();
    let user_gid: GroupId = get_gid();
    let mode = get_file_mode(&path)?;
    let can_user_read = mode & 0o400 != 0;
    let can_group_read = mode & 0o040 != 0;
    Ok(
        (user_uid == file_uid && can_user_read) || (user_gid == file_gid && can_group_read),
    )
}

pub struct Shell {
    pub variables: HashMap<String, String>,
    pub is_login: bool,
    pub argv: Vec<String>,
    pub user: UserId,
    pub status: ExitCode,
    pub home: PathBuf,
    pub path: Vec<PathBuf>,
    pub prompt: String,
    pub cwd: PathBuf,
}

impl Shell {
    pub fn new() -> Result<Shell> {
        let user = get_uid();
        let path = var("PATH")
            .unwrap_or(String::from("/usr/bin"))
            .split(':')
            .map(PathBuf::from)
            .collect();
        let argv = args().collect();
        Ok(Shell {
            variables: HashMap::new(),
            is_login: Self::is_login(&argv),
            argv,
            user,
            status: 0,
            path,
            home: get_home_dir(user)?,
            cwd: get_current_dir()?,
            prompt: Self::get_prompt(user)
        })
    }

    pub fn interpret(&self, path: &PathBuf) -> Result<()> {
        let fdi = open_file(path, libc::O_RDONLY)?;
        let content = read_file(fdi)?;
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
            write_to_file(1, arg).ok();
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
    fn get_prompt(user: UserId) -> String {
        let hostname = get_hostname().unwrap_or("hostname".to_string());
        let suffix = if user == 0 { "#" } else { "%" };
        hostname.add(suffix)
    }

    /// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
    pub fn interpret_rc(&self, rc_name: &str) -> Result<()> {
        let mut rc_file = self.home.clone();
        rc_file.push(rc_name);
        return if check_file(&rc_file)? {
            self.interpret(&rc_file)
        } else {
            Ok(())
        };
    }

    pub fn interact(&self) -> Result<()> {
        let prompt = self.prompt.clone();
        write_to_file(1, prompt)?;
        let input = read_file(0)?;
        if input.contains("pwd") {
            let cwd = self.cwd.clone();
            let cwd = cwd.to_str().ok_or(Error::InvalidUnicode)?;
            let cwd = cwd.to_string();
            write_to_file(1, cwd)?;
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
