use native::*;
use native::file_stat::*;
use native::users::*;

use shell::Shell;

pub mod native;
pub mod shell;

fn main() {
    match Shell::new() {
        Err(reason) => write_exit(4, &format!("{}", reason)),
        Ok(mut shell) => {
            shell.on_start().ok();
            if shell.argv.len() > 1 {
                if let Err(reason) = shell.handle_arguments() {
                    let error = format!("{}\n", reason);
                    write_exit(5, &error);
                }
            } else {
                if let Err(reason) = shell.interact() {
                    let error = format!("{}\n", reason);
                    write_exit(6, &error);
                }
            }
            if shell.is_login {
                shell.interpret_rc(".logout").ok();
            }
        }
    }
}
