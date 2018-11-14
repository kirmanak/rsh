extern crate libc;

use std::env::{args, var, VarError};
use std::fs::{File, read_dir};
use std::io::{BufRead, BufReader, stdin, stdout};
use std::io::Write;
use std::path::PathBuf;

use libc::{execve, fork};

fn main() {
    if args().len() > 1 {
        for open_result in args().skip(1).map(File::open) {
            match open_result {
                Ok(file) => {
                    let mut reader = BufReader::new(file);
                    read_and_execute(&mut reader, false);
                }
                Err(error) => {
                    eprintln!("{}", error);
                }
            }
        }
    } else {
        let stdin = stdin();
        let mut reader = stdin.lock();
        read_and_execute(&mut reader, true);
    }
}

fn get_prompt() -> &'static str {
    if let Ok(path) = find_program("hostname") {
        "program found%"
    } else {
        "hostname%"
    }
}

fn find_program(name: &str) -> Result<PathBuf, VarError> {
    for dir_path in var("PATH")?.split(':') {
        if let Ok(files) = read_dir(dir_path) {
            for file in files.filter(Result::is_ok).map(Result::unwrap) {
                if file.file_name().to_str().unwrap().eq(name) {
                    return Ok(file.path());
                }
            }
        }
    }
    Err(VarError::NotPresent)
}

fn read_and_execute(reader: &mut BufRead, interactive: bool) {
    if interactive {
        interact(reader, get_prompt());
    } else {
        interpret(reader)
    }
}

fn interact(reader: &mut BufRead, prompt: &str) {
    print!("{} ", prompt);
    stdout().flush().unwrap();
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => {
                execute(&line)
            }
            Err(error) => {
                eprintln!("{}", error);
            }
        }
        print!("{} ", prompt);
        stdout().flush().unwrap();
    }
}

fn interpret(reader: &mut BufRead) {
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => {
                execute(&line)
            }
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    }
}

fn execute(line: &str) {
    println!("{}", line);
}

fn fork_child() {
    let id;
    unsafe {
        id = fork();
    }
    let filename = "/usr/bin/ls";
    let length = filename.len();
    match id {
        -1 => (),
        0 => (),
        _ => unsafe {
            let filename = filename.as_ptr() as *const i8;
            let argv = [filename, std::ptr::null()].as_ptr();
            let envp = [std::ptr::null()].as_ptr();
            println!("{}", execve(filename, argv, envp));
            println!("{}", String::from_raw_parts(filename as *mut u8, length, length));
        }
    }
}
