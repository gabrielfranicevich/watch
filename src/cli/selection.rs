use std::io::{self, BufRead, Write};

/// Read a one-based selection from a list of options.
/// Returns the zero-based index, or None when input reaches EOF or the user cancels.
pub fn read_selection<R, W>(
    input: &mut R,
    output: &mut W,
    option_count: usize,
) -> io::Result<Option<usize>>
where
    R: BufRead,
    W: Write,
{
    if option_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot select from an empty list",
        ));
    }

    loop {
        write!(
            output,
            "Select an entry (1-{option_count}, or q to cancel): "
        )?;
        output.flush()?;

        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }

        let value = line.trim();
        if value.eq_ignore_ascii_case("q") {
            return Ok(None);
        }

        match value.parse::<usize>() {
            Ok(number) if (1..=option_count).contains(&number) => return Ok(Some(number - 1)),
            _ => writeln!(
                output,
                "Invalid selection. Enter a number from 1 to {option_count}, or q to cancel."
            )?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn converts_one_based_input_to_zero_based_index() {
        let mut input = Cursor::new("2\n");
        let mut output = Vec::new();
        assert_eq!(read_selection(&mut input, &mut output, 3).unwrap(), Some(1));
    }

    #[test]
    fn retries_invalid_input() {
        let mut input = Cursor::new("abc\n0\n4\n3\n");
        let mut output = Vec::new();
        assert_eq!(read_selection(&mut input, &mut output, 3).unwrap(), Some(2));
        assert_eq!(
            String::from_utf8(output)
                .unwrap()
                .matches("Invalid selection")
                .count(),
            3
        );
    }

    #[test]
    fn allows_cancellation() {
        let mut input = Cursor::new("q\n");
        let mut output = Vec::new();
        assert_eq!(read_selection(&mut input, &mut output, 2).unwrap(), None);
    }

    #[test]
    fn returns_none_on_end_of_input() {
        let mut input = Cursor::new("");
        let mut output = Vec::new();
        assert_eq!(read_selection(&mut input, &mut output, 2).unwrap(), None);
    }

    #[test]
    fn rejects_empty_option_lists() {
        let mut input = Cursor::new("1\n");
        let mut output = Vec::new();
        assert_eq!(
            read_selection(&mut input, &mut output, 0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
