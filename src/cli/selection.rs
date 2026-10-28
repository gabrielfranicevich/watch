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
    false,
  )?;

  loop {
    match read_key()? {
      Key::Enter => match parse_selection_input(&input, labels.len(), multi) {
        Some(indices) if !indices.is_empty() => {
          writeln!(output)?;
          return Ok(SelectionOutcome::Selected(indices));
        }
        _ => {
          input.clear();
          line = format!("Invalid; enter 1-{}: ", labels.len());
          previous_width = 0;
          write_prompt(&mut output, &line, &input, &mut previous_width)?;
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
          true,
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
          true,
        )?;
      }
      Key::Left | Key::Right => {}
      Key::Character(character) if character.is_ascii_digit() || (multi && character == ' ') => {
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
  redraw: bool,
) -> io::Result<()> {
  let range = page_bounds(view.labels.len(), view.page);
  let index_width = view.labels.len().to_string().len();
  let visible_rows = if view.page_count > 1 {
    PAGE_SIZE
  } else {
    range.len()
  };
  if redraw {
    write!(output, "\r\x1b[{visible_rows}A")?;
  }
  for offset in 0..visible_rows {
    if redraw {
      write!(output, "\r\x1b[2K")?;
    }
    if let Some(label) = view.labels.get(range.start + offset) {
      write!(
        output,
        "{:>index_width$}. {label}",
        range.start + offset + 1
      )?;
    }
    write!(output, "\r\n")?;
  }

  let line = prompt_line(view.prompt, view.page, view.page_count);
  if redraw {
    write!(output, "\r\x1b[2K")?;
  }
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
    VK_C if control_key_state & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0 => Key::Cancel,
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
  input_handle: *mut std::ffi::c_void,
  original_input_mode: u32,
  owns_input_handle: bool,
  output_handle: *mut std::ffi::c_void,
  original_output_mode: u32,
  owns_output_handle: bool,
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
    return Err(io::Error::new(
      io::ErrorKind::Unsupported,
      "interactive terminal input is unavailable",
    ));
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
  const STD_INPUT_HANDLE: u32 = (-10i32) as u32;
  const STD_OUTPUT_HANDLE: u32 = (-11i32) as u32;
  let (input_handle, owns_input_handle) = console_handle(STD_INPUT_HANDLE, "CONIN$")?;
  let (output_handle, owns_output_handle) = match console_handle(STD_OUTPUT_HANDLE, "CONOUT$") {
    Ok(handle) => handle,
    Err(error) => {
      if owns_input_handle {
        let _ = unsafe { CloseHandle(input_handle) };
      }
      return Err(error);
    }
  };
  let mut original_input_mode = 0u32;
  if unsafe { GetConsoleMode(input_handle, &mut original_input_mode) } == 0 {
    close_if_owned(input_handle, owns_input_handle);
    close_if_owned(output_handle, owns_output_handle);
    return Err(console_unavailable());
  }
  let mut original_output_mode = 0u32;
  if unsafe { GetConsoleMode(output_handle, &mut original_output_mode) } == 0 {
    close_if_owned(input_handle, owns_input_handle);
    close_if_owned(output_handle, owns_output_handle);
    return Err(console_unavailable());
  }
  const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
  const ENABLE_LINE_INPUT: u32 = 0x0002;
  const ENABLE_ECHO_INPUT: u32 = 0x0004;
  const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
  let raw_mode =
    original_input_mode & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT);
  if unsafe { SetConsoleMode(input_handle, raw_mode) } == 0 {
    close_if_owned(input_handle, owns_input_handle);
    close_if_owned(output_handle, owns_output_handle);
    return Err(console_unavailable());
  }
  if unsafe {
    SetConsoleMode(
      output_handle,
      original_output_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    )
  } == 0
  {
    let error = console_unavailable();
    let _ = unsafe { SetConsoleMode(input_handle, original_input_mode) };
    close_if_owned(input_handle, owns_input_handle);
    close_if_owned(output_handle, owns_output_handle);
    return Err(error);
  }
  Ok(RawModeGuard {
    input_handle,
    original_input_mode,
    owns_input_handle,
    output_handle,
    original_output_mode,
    owns_output_handle,
  })
}

#[cfg(windows)]
fn console_handle(
  standard_handle_id: u32,
  console_device: &str,
) -> io::Result<(*mut std::ffi::c_void, bool)> {
  let standard_handle = unsafe { GetStdHandle(standard_handle_id) };
  let mut mode = 0u32;
  if !standard_handle.is_null()
    && standard_handle as isize != -1
    && unsafe { GetConsoleMode(standard_handle, &mut mode) } != 0
  {
    return Ok((standard_handle, false));
  }
  open_console_device(console_device).map(|handle| (handle, true))
}

#[cfg(windows)]
fn open_console_device(device: &str) -> io::Result<*mut std::ffi::c_void> {
  const GENERIC_READ: u32 = 0x8000_0000;
  const GENERIC_WRITE: u32 = 0x4000_0000;
  const FILE_SHARE_READ: u32 = 0x0000_0001;
  const FILE_SHARE_WRITE: u32 = 0x0000_0002;
  const OPEN_EXISTING: u32 = 3;
  let name: Vec<u16> = device.encode_utf16().chain(Some(0)).collect();
  let handle = unsafe {
    CreateFileW(
      name.as_ptr(),
      GENERIC_READ | GENERIC_WRITE,
      FILE_SHARE_READ | FILE_SHARE_WRITE,
      std::ptr::null_mut(),
      OPEN_EXISTING,
      0,
      std::ptr::null_mut(),
    )
  };
  if handle.is_null() || handle as isize == -1 {
    Err(console_unavailable())
  } else {
    Ok(handle)
  }
}

#[cfg(windows)]
fn close_if_owned(handle: *mut std::ffi::c_void, owned: bool) {
  if owned {
    let _ = unsafe { CloseHandle(handle) };
  }
}

#[cfg(windows)]
fn console_unavailable() -> io::Error {
  io::Error::new(
    io::ErrorKind::Unsupported,
    "interactive console input or output is unavailable",
  )
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
      let _ = unsafe { SetConsoleMode(self.input_handle, self.original_input_mode) };
      let _ = unsafe { SetConsoleMode(self.output_handle, self.original_output_mode) };
      close_if_owned(self.input_handle, self.owns_input_handle);
      close_if_owned(self.output_handle, self.owns_output_handle);
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
  fn CreateFileW(
    file_name: *const u16,
    desired_access: u32,
    share_mode: u32,
    security_attributes: *mut std::ffi::c_void,
    creation_disposition: u32,
    flags_and_attributes: u32,
    template_file: *mut std::ffi::c_void,
  ) -> *mut std::ffi::c_void;
  fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
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
  const KEY_EVENT: u16 = 0x0001;

  let handle = open_console_device("CONIN$")?;
  let result = loop {
    let mut record = WindowsInputRecord {
      event_type: 0,
      event: WindowsInputEvent { _padding: [0; 4] },
    };
    let mut records_read = 0u32;
    let success = unsafe { ReadConsoleInputW(handle, &mut record, 1, &mut records_read) };
    if success == 0 {
      break Err(io::Error::last_os_error());
    }
    if records_read == 0 {
      break Err(io::Error::new(
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
      break Ok(key);
    }
  };
  let _ = unsafe { CloseHandle(handle) };
  result
}

#[cfg(not(any(unix, windows)))]
fn read_key_platform() -> io::Result<Key> {
  Err(io::Error::new(
    io::ErrorKind::Unsupported,
    "raw terminal input is not supported on this platform",
  ))
}

#[cfg(test)]
#[path = "../../tests/unit/cli/selection.rs"]
mod tests;
