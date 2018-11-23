use std::path::PathBuf;
use std::collections::HashMap;
use std::env::{args, var, vars};
use std::ffi::OsString;
use std::iter::once;

use super::*;

/// The structure represents the state of a shell. First of all, it stores variables.
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
    /// Constructs a new shell.
    /// It performs many syscalls to initialize all variables.
    /// Since a few of these calls can fail, the function returns Result.
    pub fn new() -> Result<Self> {
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
            prompt: get_prompt(user),
        })
    }

    /// The function opens a file on the provided path if any and tries to interpret this file.
    /// All changes in shell variables are saved!
    /// It is recommended to call this function in a clone of the current shell.
    pub fn interpret(&mut self, path: &PathBuf) -> Result<()> {
        let fdi = open_file(path, libc::O_RDONLY)?;
        let content = read_file(fdi)?;
        for line in content.lines() {
            self.parse(line)?;
        }
        Ok(())
    }

    /// Parses the command and executes it.
    /// Returns true if reading should be stopped.
    fn parse(&mut self, line: &str) -> Result<bool> {
        let mut arguments = line.split_whitespace();
        let environment: Vec<String> = vars()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect();
        let mut envp: Vec<&str> = environment.iter().map(|s| s.as_str()).collect();
        let mut argument;
        loop {
            argument = match arguments.next() {
                Some(value) => value,
                None => return Err(Error::NotFound),
            };
            if argument.contains('=') {
                envp.push(argument);
            } else {
                break;
            }
        }
        match argument {
            "exit" => Ok(true),
            "pwd" => {
                let cwd = self.cwd.clone();
                let cwd = cwd.to_str().ok_or(Error::InvalidUnicode)?;
                write_to_file(1, &format!("{}\n", cwd))?;
                Ok(false)
            }
            _ => {
                self.status = fork_process(|| {
                    let path = match self.find_path(argument) {
                        None => return Error::NotFound,
                        Some(value) => value,
                    };
                    let arguments = once(argument).chain(arguments).collect();
                    execute(&path, arguments, envp)
                })?;
                Ok(false)
            }
        }
    }

    /// Iterates over the PATH variable contents looking for the program
    fn find_path(&self, name: &str) -> Option<PathBuf> {
        if name.contains('/') {
            Some(PathBuf::from(name))
        } else {
            let name = OsString::from(name);
            for path in &self.path {
                if let Ok(dir) = path.read_dir() {
                    for entry in dir {
                        if let Ok(entry) = entry {
                            if entry.file_name() == name {
                                return Some(entry.path());
                            }
                        }
                    }
                }
            }
            None
        }
    }

    /// Checks whether we're the login shell or not
    fn is_login(args: &Vec<String>) -> bool {
        match args.len() {
            // first argument MUST be present
            0 => write_exit(7, "Something went REALLY wrong"),
            1 => args[0].starts_with('-'), // we had no arguments and started as -<something>,
            2 => args[1].eq(&"-l".to_string()), // we had only one argument - "-l",
            _ => false,
        }
    }

    /// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
    pub fn interpret_rc(&mut self, rc_name: &str) -> Result<()> {
        let mut rc_file = self.home.clone();
        rc_file.push(rc_name);
        return if check_file(&rc_file)? {
            self.interpret(&rc_file)
        } else {
            Ok(())
        };
    }

    /// Starts interactive shell which prints prompt and waits for user's input.
    pub fn interact(&mut self) -> Result<()> {
        loop {
            write_to_file(1, &self.prompt)?;
            let input = read_line(0)?;
            if self.parse(&input)? {
                break;
            }
        }
        Ok(())
    }

    /// Reads initial scripts
    pub fn on_start(&mut self) -> Result<()> {
        if self.is_login {
            self.interpret(&PathBuf::from("/etc/.login"))?;
            self.interpret_rc(".cshrc")?;
            self.interpret_rc(".login")?;
        } else {
            self.interpret_rc(".cshrc")?;
        }
        Ok(())
    }

    /// Iterates over arguments given to the shell
    pub fn handle_arguments(&mut self) -> Result<()> {
        let args: Vec<String> = self.argv.iter().skip(1).cloned().collect();
        for arg in args {
            if arg == "-" {
                self.interact()?;
            } else if arg.starts_with("-") {
                continue;
            } else {
                self.interpret(&PathBuf::from(arg))?;
            }
        }
        Ok(())
    }
}

/// Gets text for prompt from the system
fn get_prompt(user: UserId) -> String {
    let hostname = get_hostname().unwrap_or(String::from("hostname"));
    let suffix = if user == 0 { "#" } else { "%" };
    format!("{}{} ", hostname, suffix)
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
