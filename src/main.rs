extern crate libc;

use std::env::{args, var};
use std::fs::{File, read_dir};
use std::io::{BufRead, BufReader, stdin, stdout, Write};
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
                    println_error(error.to_string().as_str())
                }
            }
        }
    } else {
        let stdin = stdin();
        let mut reader = stdin.lock();
        read_and_execute(&mut reader, true);
    }
}

fn get_prompt() -> String {
    let default = "hostname";
    if let Some(path) = find_program("hostname") {
        path.to_str().unwrap_or(default).to_string()
    } else {
        default.to_string()
    }
}

fn find_program(name: &str) -> Option<PathBuf> {
    for dir_path in var("PATH").ok()?.split(':') {
        if let Ok(files) = read_dir(dir_path) {
            for file in files.filter(Result::is_ok).map(Result::unwrap) {
                if file.file_name().to_str().unwrap().eq(name) {
                    return Some(file.path());
                }
            }
        }
    }
    None
}

fn read_and_execute(reader: &mut BufRead, interactive: bool) {
    if interactive {
        interact(reader, get_prompt().as_str());
    } else {
        interpret(reader)
    }
}

fn interact(reader: &mut BufRead, prompt: &str) {
    print_prompt(prompt);
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => {
                parse(&line)
            }
            Err(error) => {
                println_error(error.to_string().as_str());
            }
        }
        print_prompt(prompt);
    }
}

fn interpret(reader: &mut BufRead) {
    for read_result in reader.lines() {
        match read_result {
            Ok(line) => {
                parse(&line)
            }
            Err(error) => {
                println_error(error.to_string().as_str());
                break;
            }
        }
    }
}

fn parse(line: &str) {
}

fn fork_child() {
    let id = unsafe { fork() };
    let filename = "/usr/bin/ls";
    match id {
        -1 => (),
        0 => (),
        _ => unsafe {
            let filename = filename.as_ptr() as *const i8;
            let argv = [filename, std::ptr::null()].as_ptr();
            let envp = [std::ptr::null()].as_ptr();
            execve(filename, argv, envp);
        }
    }
}

fn println_error(text: &str) {
    eprintln!("{}", text);
}

fn print_prompt(prompt: &str) {
    print!("{}% ", prompt);
    stdout().flush().unwrap();
}

