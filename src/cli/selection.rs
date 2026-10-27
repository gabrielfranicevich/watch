use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Write};

const PAGE_SIZE: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionOutcome {
    Selected(Vec<usize>),
    Back,
    Cancelled,
}

#[derive(Debug)]
pub struct BackRequested;

impl fmt::Display for BackRequested {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("return to the previous selection")
    }
}

impl Error for BackRequested {}

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

/// Paginated built-in selection for interactive terminals when no external menu
/// is available. Raw input is necessary to distinguish Backspace from an empty line.
pub fn read_paged_selection(
    labels: &[String],
    prompt: &str,
    multi: bool,
) -> io::Result<SelectionOutcome> {
    if labels.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot show a menu with no entries",
        ));
    }

    let _raw_mode = RawModeGuard::enable()?;
    let mut output = io::stdout().lock();
    let page_count = labels.len().div_ceil(PAGE_SIZE);
    let mut page = 0;
    let mut input = String::new();
    let mut line = prompt_line(prompt, page, page_count);
    let mut previous_width = 0;

    render_page(
        &mut output,
        &PageView {
            labels,
            prompt,
            page,
            page_count,
        },
        &input,
        &mut previous_width,
    )?;

    loop {
        match read_key()? {
            Key::Enter => match parse_selection_input(&input, labels.len(), multi) {
                Some(indices) if !indices.is_empty() => {
                    writeln!(output)?;
                    return Ok(SelectionOutcome::Selected(indices));
                }
                _ => {
                    writeln!(
                        output,
                        "\r\nInvalid selection. {} from 1 to {}.",
                        if multi {
                            "Choose one or more entries"
                        } else {
                            "Choose an entry"
                        },
                        labels.len()
                    )?;
                    input.clear();
                    let updated_line = prompt_line(prompt, page, page_count);
                    previous_width = 0;
                    write_prompt(&mut output, &updated_line, &input, &mut previous_width)?;
                }
            },
            Key::Backspace if input.is_empty() => {
                writeln!(output)?;
                return Ok(SelectionOutcome::Back);
            }
            Key::Backspace => {
                input.pop();
                write_prompt(&mut output, &line, &input, &mut previous_width)?;
            }
            Key::Escape | Key::Cancel => {
                writeln!(output)?;
                return Ok(SelectionOutcome::Cancelled);
            }
            Key::Character('q' | 'Q') if input.is_empty() => {
                writeln!(output)?;
                return Ok(SelectionOutcome::Cancelled);
            }
            Key::Right if input.is_empty() && page + 1 < page_count => {
                page += 1;
                input.clear();
                line = prompt_line(prompt, page, page_count);
                previous_width = 0;
                render_page(
                    &mut output,
                    &PageView {
                        labels,
                        prompt,
                        page,
                        page_count,
                    },
                    &input,
                    &mut previous_width,
                )?;
            }
            Key::Left if input.is_empty() && page > 0 => {
                page -= 1;
                input.clear();
                line = prompt_line(prompt, page, page_count);
                previous_width = 0;
                render_page(
                    &mut output,
                    &PageView {
                        labels,
                        prompt,
                        page,
                        page_count,
                    },
                    &input,
                    &mut previous_width,
                )?;
            }
            Key::Left | Key::Right => {}
            Key::Character(character)
                if character.is_ascii_digit() || (multi && character == ' ') =>
            {
                if character != ' ' || !input.is_empty() {
                    input.push(character);
                    write_prompt(&mut output, &line, &input, &mut previous_width)?;
                }
            }
            Key::Other | Key::Character(_) => {}
        }
    }
}

fn prompt_line(prompt: &str, page: usize, page_count: usize) -> String {
    let prompt = prompt.trim().trim_end_matches(':');
    if page_count > 1 {
        format!(
            "{} [page {}/{}; ←/→ page; Backspace back; Esc cancel]: ",
            prompt,
            page + 1,
            page_count
        )
    } else {
        format!("{prompt}: ")
    }
}

struct PageView<'a> {
    labels: &'a [String],
    prompt: &'a str,
    page: usize,
    page_count: usize,
}

fn render_page<W: Write>(
    output: &mut W,
    view: &PageView<'_>,
    input: &str,
    previous_width: &mut usize,
) -> io::Result<()> {
    let range = page_bounds(view.labels.len(), view.page);
    let index_width = view.labels.len().to_string().len();
    writeln!(
        output,
        "\r\n{} (page {} of {}):",
        view.prompt.trim_end(),
        view.page + 1,
        view.page_count
    )?;
    for (index, label) in view
        .labels
        .iter()
        .enumerate()
        .take(range.end)
        .skip(range.start)
    {
        writeln!(output, "{:>index_width$}. {label}", index + 1)?;
    }

    let line = prompt_line(view.prompt, view.page, view.page_count);
    *previous_width = 0;
    write_prompt(output, &line, input, previous_width)
}

fn write_prompt<W: Write>(
    output: &mut W,
    prompt: &str,
    input: &str,
    previous_width: &mut usize,
) -> io::Result<()> {
    let line = format!("{prompt}{input}");
    let width = line.chars().count();
    write!(output, "\r{line}")?;
    if *previous_width > width {
        write!(output, "{}", " ".repeat(*previous_width - width))?;
        write!(output, "\r{line}")?;
    }
    *previous_width = width;
    output.flush()
}

fn parse_selection_input(input: &str, option_count: usize, multi: bool) -> Option<Vec<usize>> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() || (!multi && tokens.len() != 1) {
        return None;
    }

    let mut indices = Vec::new();
    for token in tokens {
        let number = token.parse::<usize>().ok()?;
        if !(1..=option_count).contains(&number) {
            return None;
        }
        let index = number - 1;
        if !indices.contains(&index) {
            indices.push(index);
        }
    }
    Some(indices)
}

fn page_bounds(item_count: usize, page: usize) -> std::ops::Range<usize> {
    let start = page * PAGE_SIZE;
    start..(start + PAGE_SIZE).min(item_count)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Character(char),
    Enter,
    Backspace,
    Escape,
    Left,
    Right,
    Cancel,
    Other,
}

fn key_from_character(character: char) -> Key {
    match character {
        '\r' | '\n' => Key::Enter,
        '\u{8}' | '\u{7f}' => Key::Backspace,
        '\u{1b}' => Key::Escape,
        '\u{3}' => Key::Cancel,
        character if character.is_ascii() => Key::Character(character),
        _ => Key::Other,
    }
}


#[cfg(any(windows, test))]
fn key_from_windows_event(
    key_down: bool,
    virtual_key_code: u16,
    unicode_char: u16,
    control_key_state: u32,
) -> Option<Key> {
    if !key_down {
        return None;
    }

    const VK_RETURN: u16 = 0x0d;
    const VK_BACK: u16 = 0x08;
    const VK_ESCAPE: u16 = 0x1b;
    const VK_LEFT: u16 = 0x25;
    const VK_RIGHT: u16 = 0x27;
    const VK_C: u16 = 0x43;
    const LEFT_CTRL_PRESSED: u32 = 0x0008;
    const RIGHT_CTRL_PRESSED: u32 = 0x0004;

    let key = match virtual_key_code {
        VK_RETURN => Key::Enter,
        VK_BACK => Key::Backspace,
        VK_ESCAPE => Key::Escape,
        VK_LEFT => Key::Left,
        VK_RIGHT => Key::Right,
        VK_C if control_key_state & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0 => {
            Key::Cancel
        }
        _ => match char::from_u32(unicode_char as u32) {
            Some(character) if character != '\0' => key_from_character(character),
            _ => Key::Other,
        },
    };

    Some(key)
}

#[cfg(unix)]
struct RawModeGuard {
    original: libc::termios,
}

#[cfg(windows)]
struct RawModeGuard {
    handle: *mut std::ffi::c_void,
    original_mode: u32,
}

#[cfg(not(any(unix, windows)))]
struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> io::Result<Self> {
        enable_raw_mode()
    }
}

#[cfg(unix)]
fn enable_raw_mode() -> io::Result<RawModeGuard> {
    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(libc::STDIN_FILENO, original.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let original = unsafe { original.assume_init() };
    let mut raw = original;
    raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
    raw.c_cc[libc::VMIN] = 1;
    raw.c_cc[libc::VTIME] = 0;
    if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(RawModeGuard { original })
}

#[cfg(windows)]
fn enable_raw_mode() -> io::Result<RawModeGuard> {
    let handle = unsafe { GetStdHandle((-10i32) as u32) };
    if handle.is_null() || handle as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    let mut original_mode = 0u32;
    if unsafe { GetConsoleMode(handle, &mut original_mode) } == 0 {
        return Err(io::Error::last_os_error());
    }
    const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    let raw_mode =
        original_mode & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
    if unsafe { SetConsoleMode(handle, raw_mode) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(RawModeGuard {
        handle,
        original_mode,
    })
}

#[cfg(not(any(unix, windows)))]
fn enable_raw_mode() -> io::Result<RawModeGuard> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "raw terminal input is not supported on this platform",
    ))
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original) };
        }

        #[cfg(windows)]
        {
            let _ = unsafe { SetConsoleMode(self.handle, self.original_mode) };
        }
    }
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct WindowsKeyEvent {
    key_down: i32,
    _repeat_count: u16,
    virtual_key_code: u16,
    _virtual_scan_code: u16,
    unicode_char: u16,
    control_key_state: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
union WindowsInputEvent {
    key: WindowsKeyEvent,
    _padding: [u32; 4],
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct WindowsInputRecord {
    event_type: u16,
    event: WindowsInputEvent,
}

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetStdHandle(n_std_handle: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    fn SetConsoleMode(handle: *mut std::ffi::c_void, mode: u32) -> i32;
    fn ReadConsoleInputW(
        handle: *mut std::ffi::c_void,
        records: *mut WindowsInputRecord,
        length: u32,
        records_read: *mut u32,
    ) -> i32;
}

fn read_key() -> io::Result<Key> {
    read_key_platform()
}

#[cfg(unix)]
fn read_key_platform() -> io::Result<Key> {
    let mut byte = 0u8;
    let read = unsafe { libc::read(libc::STDIN_FILENO, (&mut byte as *mut u8).cast(), 1) };
    if read < 0 {
        return Err(io::Error::last_os_error());
    }
    if read == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "terminal input closed",
        ));
    }
    if byte != 0x1b {
        return Ok(key_from_character(byte as char));
    }

    let mut descriptor = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut descriptor, 1, 40) } <= 0 {
        return Ok(Key::Escape);
    }

    let mut next = 0u8;
    if unsafe { libc::read(libc::STDIN_FILENO, (&mut next as *mut u8).cast(), 1) } != 1 {
        return Ok(Key::Escape);
    }
    if next != b'[' {
        return Ok(Key::Escape);
    }

    loop {
        if unsafe { libc::poll(&mut descriptor, 1, 40) } <= 0 {
            return Ok(Key::Other);
        }
        let mut code = 0u8;
        if unsafe { libc::read(libc::STDIN_FILENO, (&mut code as *mut u8).cast(), 1) } != 1 {
            return Ok(Key::Other);
        }
        if (0x40..=0x7e).contains(&code) {
            return Ok(match code {
                b'C' => Key::Right,
                b'D' => Key::Left,
                _ => Key::Other,
            });
        }
    }
}

#[cfg(windows)]
fn read_key_platform() -> io::Result<Key> {
    const STD_INPUT_HANDLE: u32 = (-10i32) as u32;
    const KEY_EVENT: u16 = 0x0001;

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    loop {
        let mut record = WindowsInputRecord {
            event_type: 0,
            event: WindowsInputEvent { _padding: [0; 4] },
        };
        let mut records_read = 0u32;
        let success = unsafe { ReadConsoleInputW(handle, &mut record, 1, &mut records_read) };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        if records_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "terminal input closed",
            ));
        }
        if record.event_type != KEY_EVENT {
            continue;
        }

        let event = unsafe { record.event.key };
        if let Some(key) = key_from_windows_event(
            event.key_down != 0,
            event.virtual_key_code,
            event.unicode_char,
            event.control_key_state,
        ) {
            return Ok(key);
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn read_key_platform() -> io::Result<Key> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "raw terminal input is not supported on this platform",
    ))
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

    #[test]
    fn parses_multiple_one_based_numbers_for_multi_selection() {
        assert_eq!(parse_selection_input("1 3 5", 5, true), Some(vec![0, 2, 4]));
    }

    #[test]
    fn rejects_invalid_and_out_of_range_selection_input() {
        assert_eq!(parse_selection_input("0", 5, false), None);
        assert_eq!(parse_selection_input("6", 5, true), None);
        assert_eq!(parse_selection_input("1 2", 5, false), None);
    }

    #[test]
    fn page_bounds_cover_every_item_without_overlap() {
        assert_eq!(page_bounds(31, 0), 0..15);
        assert_eq!(page_bounds(31, 1), 15..30);
        assert_eq!(page_bounds(31, 2), 30..31);
    }

    #[test]
    fn page_prompt_only_shows_navigation_hints_for_multiple_pages() {
        assert_eq!(prompt_line("Select anime: ", 0, 1), "Select anime: ");
        assert_eq!(
            prompt_line("Select anime: ", 1, 2),
            "Select anime [page 2/2; ←/→ page; Backspace back; Esc cancel]: "
        );
    }

    #[test]
    fn maps_arrow_key_sequences() {
        assert_eq!(key_from_character('q'), Key::Character('q'));
        assert_eq!(key_from_character('\u{1b}'), Key::Escape);
    }

    #[test]
    fn maps_terminal_control_characters() {
        assert_eq!(key_from_character('\u{7f}'), Key::Backspace);
        assert_eq!(key_from_character('\u{8}'), Key::Backspace);
        assert_eq!(key_from_character('\u{3}'), Key::Cancel);
        assert_eq!(key_from_character('\u{1b}'), Key::Escape);
    }


    #[test]
    fn maps_windows_console_key_events() {
        assert_eq!(
            key_from_windows_event(true, 0x25, 0, 0),
            Some(Key::Left)
        );
        assert_eq!(
            key_from_windows_event(true, 0x27, 0, 0),
            Some(Key::Right)
        );
        assert_eq!(
            key_from_windows_event(true, 0x1b, 0, 0),
            Some(Key::Escape)
        );
        assert_eq!(
            key_from_windows_event(true, 0x08, 0, 0),
            Some(Key::Backspace)
        );
        assert_eq!(
            key_from_windows_event(true, 0x0d, 0, 0),
            Some(Key::Enter)
        );
        assert_eq!(
            key_from_windows_event(true, 0x43, 3, 0x0008),
            Some(Key::Cancel)
        );
        assert_eq!(
            key_from_windows_event(true, 0x51, 'q' as u16, 0),
            Some(Key::Character('q'))
        );
        assert_eq!(key_from_windows_event(false, 0x25, 0, 0), None);
    }
}
