extern crate dirs;
extern crate hostname;
extern crate users;

use std::collections::HashMap;
use std::env::args;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, Result, stdin, stdout, Write};
use std::io::Error;
use std::io::ErrorKind;
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use splitter::split_arguments;

mod splitter;

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
    } else {
        let stdin = stdin();
        shell.interact(&mut stdin.lock());
    }
}


/// Checks whether the file is readable and either is owned by the current user
/// or the current user's real group ID matches the file's group ID
fn check_file(path: &PathBuf) -> Result<bool> {
    let metadata = metadata(path)?;
    let file_uid = metadata.uid();
    let file_gid = metadata.gid();
    let user_uid = users::get_current_uid();
    let user_gid = users::get_current_gid();
    let can_user_read = metadata.mode() & 0o400 != 0;
    let can_group_read = metadata.mode() & 0o040 != 0;
    Ok((user_uid == file_uid && can_user_read) || (user_gid == file_gid && can_group_read))
}

pub struct Shell {
    pub variables: HashMap<String, String>,
    pub is_login: bool,
    pub home: Option<PathBuf>,
    pub path: Vec<PathBuf>,
    pub argv: Vec<String>,
    pub user: users::uid_t,
    pub cwd: Result<PathBuf>,
    pub prompt: String,
    pub status: Option<std::process::ExitStatus>,
}

impl Shell {
    pub fn new() -> Shell {
        let path = std::env::var("PATH").unwrap_or(String::from("/usr/bin"));
        let path = path.split(':').map(PathBuf::from).collect();
        let user = users::get_current_uid();
        let argv = std::env::args().collect();
        Shell {
            variables: HashMap::new(),
            is_login: Self::is_login(&argv),
            home: dirs::home_dir(),
            path,
            cwd: std::env::current_dir(),
            prompt: Self::get_prompt(user),
            argv,
            user,
            status: None,
        }
    }

    pub fn interact(&self, reader: &mut BufRead) {
        self.print_prompt();
        for read_result in reader.lines() {
            match read_result {
                Ok(line) => self.execute(&line),
                Err(error) => eprintln!("{}", &error),
            }
            self.print_prompt();
        }
    }

    /// Parses the command and executes it
    fn execute(&self, line: &str) {
        let arguments = split_arguments(line);
    }

    fn print_prompt(&self) {
        print!("{} ", self.prompt);
        stdout().flush();
    }

    /// Checks whether we're the login shell or not
    fn is_login(args: &Vec<String>) -> bool {
        match args.len() {
            0 => panic!("Something went REALLY wrong"), // first argument MUST be present
            1 => args[1].starts_with('-'), // we had no arguments and started as -<something>,
            2 => args[2].eq(&"-l".to_string()), // we had only one argument - "-l",
            _ => false
        }
    }

    /// Gets text for prompt from the system
    fn get_prompt(user: users::uid_t) -> String {
        let hostname = hostname::get_hostname().unwrap_or("hostname".to_string());
        let suffix = if user == 0 { "#" } else { "%" };
        hostname.add(suffix)
    }

    /// Interprets the provided file.
    pub fn interpret(&self, path: &PathBuf) -> Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for read_result in reader.lines() {
            match read_result {
                Ok(line) => self.execute(&line),
                Err(reason) => return Err(reason),
            }
        }
        Ok(())
    }


    /// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
    pub fn interpret_rc(&self, rc_name: &str) -> Result<()> {
        match &self.home {
            None => return Err(Error::new(ErrorKind::NotFound, "home dir is not found")),
            Some(home) => {
                let mut rc_file = home.clone();
                rc_file.push(rc_name);
                if check_file(&rc_file)? {
                    self.interpret(&rc_file)?;
                }
                Ok(())
            }
        }

    }
}
