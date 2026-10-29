use ani_cli::application::{
  choose_playback_target, PlaybackCapabilities, PlaybackMode, PlaybackPlatform, PlaybackTarget,
};

fn desktop(terminal_graphics_supported: bool) -> PlaybackCapabilities {
  PlaybackCapabilities {
    interactive_terminal: true,
    terminal_graphics_supported,
  }
}

#[test]
fn automatic_mode_uses_inline_only_when_desktop_capability_is_confirmed() {
  assert_eq!(
    choose_playback_target(
      PlaybackMode::Automatic,
      PlaybackPlatform::Desktop,
      desktop(true),
    ),
    Ok(PlaybackTarget::Inline)
  );
  assert_eq!(
    choose_playback_target(
      PlaybackMode::Automatic,
      PlaybackPlatform::Desktop,
      desktop(false),
    ),
    Ok(PlaybackTarget::External)
  );
}

#[test]
fn automatic_mode_uses_external_playback_on_android() {
  assert_eq!(
    choose_playback_target(
      PlaybackMode::Automatic,
      PlaybackPlatform::Android,
      desktop(true),
    ),
    Ok(PlaybackTarget::External)
  );
}

#[test]
fn explicit_inline_mode_fails_clearly_on_android() {
  let error = choose_playback_target(
    PlaybackMode::Inline,
    PlaybackPlatform::Android,
    desktop(true),
  )
  .unwrap_err();
  assert!(error.contains("not supported in Termux"));
}

#[test]
fn explicit_inline_mode_requires_an_interactive_terminal() {
  let error = choose_playback_target(
    PlaybackMode::Inline,
    PlaybackPlatform::Desktop,
    PlaybackCapabilities {
      interactive_terminal: false,
      terminal_graphics_supported: true,
    },
  )
  .unwrap_err();
  assert!(error.contains("interactive terminal"));
}

#[test]
fn external_mode_never_selects_inline_playback() {
  assert_eq!(
    choose_playback_target(
      PlaybackMode::External,
      PlaybackPlatform::Desktop,
      desktop(true),
    ),
    Ok(PlaybackTarget::External)
  );
}
