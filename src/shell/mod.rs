use std::path::PathBuf;
use std::collections::HashMap;
use std::env::{args, var};

use ::*;

use self::splitter::split_arguments;

mod splitter;

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
            prompt: Self::get_prompt(user),
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
            write_to_file(1, arg.as_str()).ok();
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

    /// Gets text for prompt from the system
    fn get_prompt(user: UserId) -> String {
        let mut hostname = get_hostname().unwrap_or(String::from("hostname"));
        let suffix = if user == 0 { '#' } else { '%' };
        hostname.push(suffix);
        hostname
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
        let prompt = self.prompt.as_str();
        write_to_file(1, prompt)?;
        let input = read_file(0)?;
        if input.contains("pwd") {
            let cwd = self.cwd.clone();
            let cwd = cwd.to_str().ok_or(Error::InvalidUnicode)?;
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