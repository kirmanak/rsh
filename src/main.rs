use std::path::PathBuf;

use native::*;
use native::file_stat::*;
use native::users::*;

use shell::Shell;

pub mod native;
pub mod shell;

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
                        let error = format!("{:?}: {:?}\n", path, reason);
                        write_exit(5, error.as_str());
                    }
                });
        } else {
            if let Err(reason) = shell.interact() {
                let error = format!("{:?}\n", reason);
                write_exit(6, error.as_str());
            }
        }
    } else {
        write_exit(4, "Failed to initialize the shell");
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
