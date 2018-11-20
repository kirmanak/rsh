/// Splits the provided string to slices by spaces, single or double quotes.
/// For example,
/// ```
/// let line = "echo 'first argument' 'second argument'";
/// let splitted = split_arguments(line);
/// assert_eq!(splitted, vec!["echo", "first argument", "second argument"]);
/// ```
pub fn split_arguments(line: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    for (number, symbol) in line.chars().enumerate() {
        match symbol {
            ' ' => {
                result.push(&line[start..number]);
                start = number + 1;
            },
            _ => continue
        }
    } 
    result.push(&line[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_double_quotes() {
        let line = "echo \"first argument\" \"second argument\"";
        let expected = vec!["echo", "first argument", "second argument"];
        assert_eq!(split_arguments(line), expected);
    }

    #[test]
    fn split_single_quotes() {
        let line = "echo 'first argument' 'second argument'";
        let expected = vec!["echo", "first argument", "second argument"];
        assert_eq!(split_arguments(line), expected);
    }

    #[test]
    fn split_no_quotes() {
        let line = "echo first second third fourth";
        let expected = vec!["echo", "first", "second", "third", "fourth"];
        assert_eq!(split_arguments(line), expected);
    }
}
