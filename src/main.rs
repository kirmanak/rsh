extern crate dirs;
extern crate hostname;
extern crate users;

use std::env::{args, var};
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, Error, ErrorKind, Result, stdin, stdout, Write};
use std::ops::Add;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

use splitter::split_arguments;

mod splitter;

fn main() {
    let home = dirs::home_dir().unwrap();
    if is_login() {
        interpret(&(PathBuf::from("/etc/.login")));
        interpret_rc(&home, ".cshrc");
        interpret_rc(&home, ".login");
    } else {
        interpret_rc(&home, ".cshrc");
    }
    let args: Vec<PathBuf> = args().skip(1) // skipping the name of the shell
        .filter(|arg| !arg.starts_with('-')) // filtering options
        .map(PathBuf::from)
        .collect();
    if args.len() > 0 {
        for argument in args {
            interpret(&argument).unwrap();
        }
    } else {
        let stdin = stdin();
        interact(&mut stdin.lock());
    }
}

/// Checks whether the provided rc file should be interpreted or not. If so, it interprets it.
fn interpret_rc(home: &PathBuf, rc_name: &str) -> Result<()> {
    let mut rc_file = home.clone();
    rc_file.push(rc_name);
    if check_file(&rc_file)? {
        interpret(&rc_file)?;
    }
    Ok(())
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

/// Checks whether we're the login shell or not
fn is_login() -> bool {
    let mut args = args();
    match args.len() {
        0 => panic!("Something went REALLY wrong"), // first argument MUST be present
        1 => {
            args.next()
                .unwrap()
                .starts_with('-') // we had no arguments and started as -<something>
        },
        2 => {
            args.skip(1)
                .next()
                .unwrap()
                .eq(&"-l".to_string()) // we have only one argument - "-l"
        },
        _ => false
    }
}

/// Gets text for prompt from the system
fn get_prompt() -> String {
    let hostname = hostname::get_hostname().unwrap_or("hostname".to_string());
    let uid = users::get_current_uid();
    if uid == 0 {
        hostname.add("#")
    } else {
        hostname.add("%")
    }
}

fn get_output(program: &str) -> Result<String> {
    let path = var("PATH").unwrap_or("/usr/bin".to_string());
    let output = Command::new(program).env("PATH", path).output()?;
    let stdout = output.stdout;
    let parse_result = String::from_utf8(stdout);
    match parse_result {
        Ok(stdout) => Ok(stdout),
        Err(error) => {
            Err(Error::new(ErrorKind::InvalidData, error))
        }
    }
}

fn interact(reader: &mut BufRead) {
    let prompt = get_prompt();
    print_prompt(&prompt);
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => execute(&line),
            Err(error) => eprintln!("{}", &error),
        }
        print_prompt(&prompt);
    }
}

/// Interprets the provided file. Panics in case of any I/O error.
fn interpret(file_name: &PathBuf) -> Result<()> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => execute(&line),
            Err(reason) => return Err(reason),
        }
    }
    Ok(())
}

fn execute(line: &str) {
    let arguments = split_arguments(line);
}

fn fork_child() {
    let output = Command::new("ls").arg("-l").arg("-a").output().expect("ls failed to start");
    println!("{}", &String::from_utf8(output.stdout).unwrap());

}

fn print_prompt(prompt: &str) {
    print!("{} ", prompt);
    stdout().flush().unwrap();
}

