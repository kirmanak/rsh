extern crate libc;

use std::env::args;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Result, stdin, stdout, Write};
use std::process::Command;

fn main() {
    let args: Vec<String> = args().skip(1) // skipping the name of the shell
        .filter(|arg| !arg.starts_with('-')) // filtering unsupported options
        .collect();
    if args.len() > 0 {
        iterate_through_args(&args);
    } else {
        let stdin = stdin();
        interact(&mut stdin.lock());
    }
}

fn check_file(name: &str) -> bool {
    true
}

fn iterate_through_args(args: &Vec<String>) {
    for argument in args {
        let open_result = File::open(&argument);
        match open_result {
            Ok(file) => {
                interpret(&mut BufReader::new(file));
            }
            Err(error) => {
                let text = format!("{}: {}", &argument, &error);
                println_error(text.as_str());
                break;
            }
        }
    }
}

fn is_login() -> bool {
    let mut args = args();
    if args.next().unwrap().starts_with('-') {
        args.len() == 0 //we were invoked as -<something> and we had no arguments
    } else if args.len() == 1 {
        args.next().unwrap().eq(&"-l".to_string()) // or we were invoked only with the -l flag
    } else {
        false
    }
}

fn get_prompt() -> String {
    get_output("hostname")
        .unwrap_or("hostname".to_string())
        .trim()
        .to_string()
}

fn get_output(program: &str) -> Result<String> {
    let output = Command::new(program).output()?;
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
            Ok(line) => {
                parse(&line)
            }
            Err(error) => {
                println_error(error.to_string().as_str());
            }
        }
        print_prompt(&prompt);
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
    let output = Command::new("ls").arg("-l").arg("-a").output().expect("ls failed to start");
    println!("{}", &String::from_utf8(output.stdout).unwrap());

}

fn println_error(text: &str) {
    eprintln!("{}", text);
}

fn print_prompt(prompt: &str) {
    print!("{}% ", prompt);
    stdout().flush().unwrap();
}

