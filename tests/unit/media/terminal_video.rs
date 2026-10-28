use super::*;

#[test]
fn detects_kitty_protocol_for_known_terminal_families() {
  assert_eq!(
    TerminalVideoBackend::detect(
      None,
      Some("Ghostty"),
      Some("xterm-ghostty"),
      false,
      false,
      false
    ),
    Ok(TerminalVideoBackend::Kitty)
  );
  assert_eq!(
    TerminalVideoBackend::detect(None, None, Some("xterm-kitty"), true, false, false),
    Ok(TerminalVideoBackend::Kitty)
  );
  assert_eq!(
    TerminalVideoBackend::detect(
      None,
      Some("WezTerm"),
      Some("xterm-256color"),
      false,
      true,
      false
    ),
    Ok(TerminalVideoBackend::Kitty)
  );
}

#[test]
fn detects_sixel_for_known_terminal_families() {
  assert_eq!(
    TerminalVideoBackend::detect(None, None, Some("foot"), false, false, false),
    Ok(TerminalVideoBackend::Sixel)
  );
  assert_eq!(
    TerminalVideoBackend::detect(None, Some("Contour"), Some("contour"), false, false, false),
    Ok(TerminalVideoBackend::Sixel)
  );
  assert_eq!(
    TerminalVideoBackend::detect(None, None, Some("xterm-sixel"), false, false, false),
    Ok(TerminalVideoBackend::Sixel)
  );
}

#[test]
fn detects_windows_terminal_from_its_session_variable() {
  assert_eq!(
    TerminalVideoBackend::detect(
      None,
      Some("WindowsTerminal"),
      Some("xterm-256color"),
      false,
      false,
      true
    ),
    Ok(TerminalVideoBackend::Sixel)
  );
}

#[test]
fn unknown_terminal_falls_back_to_true_color_cells() {
  assert_eq!(
    TerminalVideoBackend::detect(
      None,
      Some("generic-shell"),
      Some("xterm-256color"),
      false,
      false,
      false
    ),
    Ok(TerminalVideoBackend::TrueColor)
  );
}

#[test]
fn configured_backend_overrides_terminal_detection() {
  assert_eq!(
    TerminalVideoBackend::detect(
      Some("SIXEL"),
      Some("Ghostty"),
      Some("xterm-ghostty"),
      true,
      true,
      false
    ),
    Ok(TerminalVideoBackend::Sixel)
  );
  assert_eq!(
    TerminalVideoBackend::detect(
      Some("tct"),
      Some("Kitty"),
      Some("xterm-kitty"),
      true,
      false,
      false
    ),
    Ok(TerminalVideoBackend::TrueColor)
  );
}

#[test]
fn invalid_configured_backend_returns_a_clear_error() {
  let error = TerminalVideoBackend::detect(Some("not-a-backend"), None, None, false, false, false)
    .unwrap_err();
  assert!(error.contains("ANI_CLI_TERMINAL_BACKEND"));
}

#[test]
fn windows_defaults_to_sixel_instead_of_nonportable_tct() {
  assert_eq!(
    default_backend(TerminalVideoBackend::TrueColor, None, true),
    TerminalVideoBackend::Sixel
  );
  assert_eq!(
    default_backend(TerminalVideoBackend::TrueColor, Some("auto"), true),
    TerminalVideoBackend::Sixel
  );
  assert_eq!(
    default_backend(TerminalVideoBackend::TrueColor, Some("tct"), true),
    TerminalVideoBackend::TrueColor
  );
  assert_eq!(
    default_backend(TerminalVideoBackend::TrueColor, None, false),
    TerminalVideoBackend::TrueColor
  );
  assert_eq!(
    default_backend(TerminalVideoBackend::Kitty, None, true),
    TerminalVideoBackend::Kitty
  );
}

#[test]
fn windows_does_not_fall_back_to_tct_when_sixel_is_missing() {
  let error = TerminalVideoBackend::Sixel
    .mpv_flags_for_platform("Available video outputs:\n  gpu\n  tct\n", true)
    .unwrap_err();
  assert!(error.contains("does not provide a usable terminal video output driver"));
  assert!(error.contains("Sixel output for Windows Terminal"));

  let flags = TerminalVideoBackend::Sixel
    .mpv_flags_for_platform("Available video outputs:\n  gpu\n  sixel\n  tct\n", true)
    .unwrap();
  assert!(flags.contains(&"--vo=sixel".to_owned()));
}

#[test]
fn selects_requested_graphics_driver_when_compiled_in() {
  let flags = TerminalVideoBackend::Kitty
    .mpv_flags("Available video outputs:\n  tct\n  kitty\n")
    .unwrap();
  assert!(flags.contains(&"--vo=kitty".to_owned()));
  assert!(flags.contains(&"--force-window=no".to_owned()));
  assert!(!flags.iter().any(|flag| flag == "--terminal=no"));
}

#[test]
fn falls_back_to_tct_on_non_windows_only_when_mpv_lists_it() {
  let flags = TerminalVideoBackend::Sixel
    .mpv_flags_for_platform("Available video outputs:\n  gpu\n  tct\n", false)
    .unwrap();
  assert!(flags.contains(&"--vo=tct".to_owned()));

  let error = TerminalVideoBackend::Kitty
    .mpv_flags_for_platform("Available video outputs:\n  gpu\n  null\n", false)
    .unwrap_err();
  assert!(error.contains("usable terminal video output driver"));
}
