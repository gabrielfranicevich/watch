use super::*;

#[test]
fn parses_query_and_options_in_either_order() {
  let cli = Cli::parse(["--download", "Frieren", "--episode", "5-6"]).unwrap();
  assert_eq!(cli.query.as_deref(), Some("Frieren"));
  assert!(cli.download);
  assert_eq!(cli.episode_range.as_deref(), Some("5-6"));
}

#[test]
fn joins_multiword_query() {
  let cli = Cli::parse(["blue", "lock", "-q", "720p"]).unwrap();
  assert_eq!(cli.query.as_deref(), Some("blue lock"));
  assert_eq!(cli.quality.as_deref(), Some("720p"));
}

#[test]
fn rejects_unknown_options() {
  let error = Cli::parse(["--not-real"]).unwrap_err();
  assert!(error.to_string().contains("unknown option"));
}

#[test]
fn rejects_missing_option_values() {
  assert!(
    Cli::parse(["--episode"])
      .unwrap_err()
      .to_string()
      .contains("requires a value")
  );
}

#[test]
fn parses_last_episode_sentinel() {
  assert_eq!(
    Cli::parse(["--episode", "-1"])
      .unwrap()
      .episode_range
      .as_deref(),
    Some("-1")
  );
}

#[test]
fn rejects_conflicting_interactive_menus() {
  assert!(Cli::parse(["--rofi", "--dmenu"]).is_err());
}

#[test]
fn last_source_flag_wins_when_continue_and_countdown_are_both_given() {
  let countdown = Cli::parse(["--continue", "--nextep-countdown"]).unwrap();
  assert_eq!(countdown.source_override, Some(SourceOverride::NextEpisode));

  let history = Cli::parse(["--nextep-countdown", "--continue"]).unwrap();
  assert_eq!(history.source_override, Some(SourceOverride::History));
}

#[test]
fn parses_one_based_select_nth_index() {
  let cli = Cli::parse(["-S", "2", "frieren"]).unwrap();
  assert!(cli.select_nth);
  assert_eq!(cli.select_index, Some(2));
  assert_eq!(cli.query.as_deref(), Some("frieren"));
}

#[test]
fn rejects_invalid_select_nth_index() {
  assert!(Cli::parse(["--select-nth"]).is_err());
  assert!(Cli::parse(["--select-nth", "0"]).is_err());
  assert!(Cli::parse(["--select-nth", "abc"]).is_err());
}

#[test]
fn supports_help_and_version_flags() {
  assert!(Cli::parse(["--help"]).unwrap().help);
  assert!(Cli::parse(["-V"]).unwrap().version);
}

#[test]
fn parses_inline_terminal_playback_and_alias() {
  assert!(Cli::parse(["--inline", "Frieren"]).unwrap().inline_playback);
  assert!(Cli::parse(["Frieren", "--here"]).unwrap().inline_playback);
}

#[test]
fn rejects_inline_playback_conflicts() {
  assert!(Cli::parse(["--inline", "--download"]).is_err());
  assert!(Cli::parse(["--inline", "--vlc"]).is_err());
  assert!(Cli::parse(["--syncplay", "--here"]).is_err());
}
#[test]
fn preserves_legacy_operational_flags() {
  let cli = Cli::parse([
    "--delete",
    "--logview",
    "--skip",
    "--no-detach",
    "--exit-after-play",
  ])
  .unwrap();

  assert!(cli.delete_history);
  assert!(cli.logview);
  assert!(cli.skip_intro);
  assert!(cli.no_detach);
  assert!(cli.exit_after_play);
}

#[test]
fn short_episode_range_alias_matches_long_option() {
  let long = Cli::parse(["--episode", "2-4"]).unwrap();
  let short = Cli::parse(["-r", "2-4"]).unwrap();

  assert_eq!(long.episode_range, short.episode_range);
}

#[test]
fn rejects_non_positive_select_nth_values() {
  for value in ["0", "-1", "not-a-number"] {
    assert!(
      Cli::parse(["--select-nth", value]).is_err(),
      "expected --select-nth {value} to be rejected"
    );
  }
}

#[test]
fn keeps_multiword_query_when_options_are_interspersed() {
  let cli = Cli::parse([
    "Frieren",
    "--download",
    "Beyond",
    "--quality",
    "720p",
    "Journey",
  ])
  .unwrap();

  assert_eq!(cli.query.as_deref(), Some("Frieren Beyond Journey"));
  assert_eq!(cli.quality.as_deref(), Some("720p"));
  assert!(cli.download);
}
