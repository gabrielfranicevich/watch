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
fn changing_pages_clears_the_previous_menu_in_place() {
  let labels: Vec<_> = (1..=31).map(|number| format!("Anime {number}")).collect();
  let mut output = Vec::new();
  let mut previous_width = 0;
  for page in 0..=1 {
    render_page(
      &mut output,
      &PageView {
        labels: &labels,
        prompt: "Select anime: ",
        page,
        page_count: 3,
      },
      "",
      &mut previous_width,
      page != 0,
    )
    .unwrap();
  }
  let output = String::from_utf8(output).unwrap();
  assert!(output.contains("\r\x1b[15A\r\x1b[2K16. Anime 16\r\n"));
  assert!(!output.contains("\x1b[J"));
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
  assert_eq!(key_from_windows_event(true, 0x25, 0, 0), Some(Key::Left));
  assert_eq!(key_from_windows_event(true, 0x27, 0, 0), Some(Key::Right));
  assert_eq!(key_from_windows_event(true, 0x1b, 0, 0), Some(Key::Escape));
  assert_eq!(
    key_from_windows_event(true, 0x08, 0, 0),
    Some(Key::Backspace)
  );
  assert_eq!(key_from_windows_event(true, 0x0d, 0, 0), Some(Key::Enter));
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
