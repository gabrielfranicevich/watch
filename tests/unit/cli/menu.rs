use super::*;

#[test]
fn text_prompt_trims_user_input_and_rejects_empty_output() {
  assert_eq!(
    parse_prompt_value("  Frieren: Beyond Journey's End \n"),
    Some("Frieren: Beyond Journey's End".to_owned())
  );
  assert_eq!(parse_prompt_value("\n \t"), None);
}

#[test]
fn fzf_text_prompt_falls_back_to_terminal_input() {
  assert_eq!(
    prompt_text("Search anime: ", Some("fzf")).unwrap(),
    TextPromptOutcome::Unavailable
  );
  assert_eq!(
    prompt_text("Search anime: ", None).unwrap(),
    TextPromptOutcome::Unavailable
  );
}

#[test]
fn maps_prefixed_menu_lines_to_zero_based_indices() {
  let items = vec!["Frieren".to_owned(), "Blue Lock".to_owned()];
  assert_eq!(parse_selected_index("2\tBlue Lock", &items), Some(1));
  assert_eq!(parse_selected_index("1\tFrieren", &items), Some(0));
}

#[test]
fn resolves_plain_custom_menu_output() {
  let items = vec!["Frieren".to_owned(), "Blue Lock".to_owned()];
  assert_eq!(parse_selected_index("Blue Lock", &items), Some(1));
}

#[test]
fn rejects_invalid_menu_indices() {
  let items = vec!["Frieren".to_owned()];
  assert_eq!(parse_selected_index("0\tFrieren", &items), None);
  assert_eq!(parse_selected_index("2\tOther", &items), None);
}
