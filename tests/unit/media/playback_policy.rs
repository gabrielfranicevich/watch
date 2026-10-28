use super::*;

#[test]
fn uses_inline_when_requested_and_available() {
  assert_eq!(choose_target(true, true, true), Ok(PlaybackTarget::Inline));
}

#[test]
fn falls_back_to_external_when_inline_is_unavailable() {
  assert_eq!(
    choose_target(true, false, true),
    Ok(PlaybackTarget::External)
  );
}

#[test]
fn prefers_external_when_inline_was_not_requested() {
  assert_eq!(
    choose_target(false, true, true),
    Ok(PlaybackTarget::External)
  );
}

#[test]
fn reports_error_when_no_target_is_available() {
  assert_eq!(
    choose_target(true, false, false),
    Err("no supported playback target is available")
  );
}
