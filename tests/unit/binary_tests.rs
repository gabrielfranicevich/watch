use super::*;
use ani_cli::providers::hianime_stream::QualityOption;

#[test]
fn selection_cancellation_is_a_successful_exit_condition() {
  let error = selection_cancelled();
  assert!(is_selection_cancelled(error.as_ref()));
  assert_eq!(error.to_string(), "selection cancelled");
}

#[test]
fn merges_provider_episode_lists_and_keeps_first_source_for_duplicate_numbers() {
  let first = ProviderEpisodes {
    source: ProviderAnime {
      provider: ProviderId::HiAnime,
      anime: Anime::new("frieren", "Frieren", "https://hianime.at/watch/frieren"),
    },
    episodes: vec![
      Episode::new("2", None, "hianime-2"),
      Episode::new("1", None, "hianime-1"),
    ],
  };
  let second = ProviderEpisodes {
    source: ProviderAnime {
      provider: ProviderId::GoGoAnime,
      anime: Anime::new("frieren", "Frieren", "https://gogoanime.by/series/frieren"),
    },
    episodes: vec![
      Episode::new("1.0", None, "gogo-1"),
      Episode::new("3", None, "gogo-3"),
    ],
  };

  let merged = merge_provider_episodes(&[first, second]);
  assert_eq!(
    merged
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["1", "2", "3"]
  );
  assert_eq!(merged[0].url, "hianime-1");
}

#[test]
fn source_quality_rank_uses_highest_available_resolution() {
  let media = ResolvedMedia {
    stream: ani_cli::domain::stream::Stream::new("https://cdn.example/video.m3u8", None, None),
    subtitle: None,
    mal_id: None,
    qualities: vec![
      QualityOption {
        label: "1080p".to_owned(),
        value: "1080p".to_owned(),
      },
      QualityOption {
        label: "720p".to_owned(),
        value: "720p".to_owned(),
      },
    ],
  };
  assert_eq!(media_quality_rank(&media), 1080);
}

#[test]
fn updating_history_keeps_existing_anime_position_and_appends_new_anime() {
  let nonce = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let path = std::env::temp_dir().join(format!(
    "ani-cli-record-history-{}-{nonce}",
    std::process::id()
  ));
  let store = HistoryStore::new(&path);
  store
    .write(&[
      HistoryEntry::new("2", "first-1", "First"),
      HistoryEntry::new("5", "second-2", "Second"),
    ])
    .unwrap();

  record_history(
    &store,
    &Anime::new(
      "first-1",
      "First Updated",
      "https://hianime.at/watch/first-1",
    ),
    &Episode::new("3", None, "https://hianime.at/watch/first-1?ep=1003"),
  )
  .unwrap();
  record_history(
    &store,
    &Anime::new("third-3", "Third", "https://hianime.at/watch/third-3"),
    &Episode::new("1", None, "https://hianime.at/watch/third-3?ep=3001"),
  )
  .unwrap();

  assert_eq!(
    store.read().unwrap(),
    vec![
      HistoryEntry::new("3", "first-1", "First Updated"),
      HistoryEntry::new("5", "second-2", "Second"),
      HistoryEntry::new("1", "third-3", "Third"),
    ]
  );
  for suffix in ["", ".lock"] {
    let _ = std::fs::remove_file({
      let mut name = path.as_os_str().to_os_string();
      name.push(suffix);
      std::path::PathBuf::from(name)
    });
  }
}

#[test]
fn default_player_falls_back_to_vlc_when_mpv_is_unavailable() {
  assert_eq!(
    select_default_player(false, false, false, false, true),
    Ok(Player::Vlc)
  );
}

#[test]
fn default_player_prefers_mpv_when_it_is_available() {
  assert_eq!(
    select_default_player(false, false, false, true, true),
    Ok(Player::Mpv)
  );
}

#[test]
fn default_player_reports_when_no_supported_player_is_available() {
  assert_eq!(
    select_default_player(false, false, false, false, false),
    Err("no supported player found; install MPV or VLC, or set ANI_CLI_PLAYER")
  );
}

#[test]
fn default_player_preserves_iina_and_flatpak_preferences() {
  assert_eq!(
    select_default_player(true, true, false, true, true),
    Ok(Player::from_configuration(
      false,
      false,
      false,
      Some("iina")
    ))
  );
  assert_eq!(
    select_default_player(false, false, true, false, true),
    Ok(Player::FlatpakMpv)
  );
}

#[test]
fn inline_mode_selects_mpv_or_flatpak_and_rejects_non_mpv_configuration() {
  assert_eq!(
    select_terminal_player(None, true, false),
    Ok(Player::FlatpakMpv)
  );
  assert_eq!(select_terminal_player(None, false, true), Ok(Player::Mpv));
  assert_eq!(
    select_terminal_player(Some("/opt/mpv"), false, false),
    Ok(Player::Configured("/opt/mpv".to_owned()))
  );
  assert_eq!(
    select_terminal_player(Some("vlc"), false, true),
    Err("--inline requires MPV; set ANI_CLI_PLAYER to an MPV executable or unset it")
  );
  assert_eq!(
    select_terminal_player(None, false, false),
    Err(
      "--inline requires MPV with terminal video output support; install MPV or set ANI_CLI_PLAYER"
    )
  );
}

#[test]
fn download_only_mode_does_not_require_a_player() {
  assert!(!player_required(true));
  assert!(player_required(false));
}

#[test]
fn flatpak_detection_uses_installed_app_directories() {
  // CI machines usually have neither installation path; the result must
  // be a pure filesystem check and not run the Flatpak command.
  let available = flatpak_mpv_installed();
  assert_eq!(
    available,
    Path::new("/var/lib/flatpak/app/io.mpv.Mpv").is_dir()
      || std::env::var_os("HOME").is_some_and(|home| {
        PathBuf::from(home)
          .join(".local/share/flatpak/app/io.mpv.Mpv")
          .is_dir()
      })
  );
}

#[test]
fn explicit_menu_flags_override_environment_and_terminal_detection() {
  let rofi = ani_cli::cli::args::Cli::parse(["--rofi"]).unwrap();
  let dmenu = ani_cli::cli::args::Cli::parse(["--dmenu"]).unwrap();
  assert_eq!(
    preferred_menu_backend(&rofi, true, Some("fzf"), true, true),
    Some("rofi".to_owned())
  );
  assert_eq!(
    preferred_menu_backend(&dmenu, false, Some("fzf"), false, true),
    Some("dmenu".to_owned())
  );
}

#[test]
fn non_terminal_input_prefers_rofi_then_dmenu_like_the_shell_version() {
  let cli = ani_cli::cli::args::Cli::default();
  assert_eq!(
    preferred_menu_backend(&cli, false, Some("fzf"), true, true),
    Some("rofi".to_owned())
  );
  assert_eq!(
    preferred_menu_backend(&cli, false, Some("fzf"), true, false),
    Some("dmenu".to_owned())
  );
}

#[test]
fn interactive_terminal_uses_configured_menu_backend() {
  let cli = ani_cli::cli::args::Cli::default();
  assert_eq!(
    preferred_menu_backend(&cli, true, Some("fzf"), true, true),
    Some("fzf".to_owned())
  );
  assert_eq!(
    preferred_menu_backend(&cli, true, Some("  "), false, false),
    None
  );
}

#[test]
fn supports_legacy_boolean_environment_flags() {
  assert!(is_enabled(Some("1")));
  assert!(!is_enabled(Some("0")));
  assert!(!is_enabled(Some("true")));
  assert!(!is_enabled(None));
}

#[test]
fn selects_source_from_cli_override_or_environment() {
  use ani_cli::cli::args::SourceOverride::{History, NextEpisode};

  assert_eq!(effective_source(None, None).unwrap(), "search");
  assert_eq!(effective_source(None, Some("history")).unwrap(), "history");
  assert_eq!(effective_source(None, Some("nextep")).unwrap(), "nextep");
  assert_eq!(
    effective_source(Some(History), Some("search")).unwrap(),
    "history"
  );
  assert_eq!(
    effective_source(Some(NextEpisode), Some("history")).unwrap(),
    "nextep"
  );
}

#[test]
fn rejects_invalid_default_source_environment_value() {
  assert!(effective_source(None, Some("bad-value")).is_err());
}

#[test]
fn updater_arguments_pin_repository_branch_and_install_root() {
  let args = updater_arguments(Path::new("/home/user/.cargo"), "develop");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(
    args
      .windows(2)
      .any(|pair| pair[0] == "--branch" && pair[1] == "develop")
  );
  assert!(
    args
      .windows(2)
      .any(|pair| pair[0] == "--root" && pair[1] == "/home/user/.cargo")
  );
  assert!(args.contains(&"https://github.com/gabrielfranicevich/watch".to_owned()));
  assert!(args.contains(&"--force".to_owned()));
  assert!(!args.contains(&"--locked".to_owned()));
}

#[test]
fn rejects_unsafe_branch_names_for_the_updater() {
  assert!(valid_branch_name("develop"));
  assert!(valid_branch_name("feature/rust-port"));
  assert!(!valid_branch_name("--help"));
  assert!(!valid_branch_name("../main"));
  assert!(!valid_branch_name(""));
}

#[test]
fn download_filename_matches_legacy_episode_stem_format() {
  assert_eq!(
    episode_file_stem("Frieren: Beyond Journey's End", "2.5"),
    "Frieren_ Beyond Journey's End Episode 2.5"
  );
  assert_eq!(episode_file_stem("A/B", "1"), "A_B Episode 1");
}

#[test]
fn filename_replaces_platform_invalid_characters() {
  assert_eq!(safe_filename("A/B: C?"), "A_B_ C_");
}

#[test]
fn history_resume_takes_precedence_over_explicit_episode_range() {
  let episodes = vec![
    Episode::new("1", None, "one"),
    Episode::new("2", None, "two"),
    Episode::new("3", None, "three"),
  ];
  let selected = select_fixed_episodes(&episodes, Some("1-2"), Some("2"))
    .unwrap()
    .unwrap();
  assert_eq!(
    selected
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["3"]
  );
}

#[test]
fn explicit_episode_range_is_used_when_not_resuming_history() {
  let episodes = vec![
    Episode::new("1", None, "one"),
    Episode::new("2", None, "two"),
    Episode::new("3", None, "three"),
  ];
  let selected = select_fixed_episodes(&episodes, Some("1-2"), None)
    .unwrap()
    .unwrap();
  assert_eq!(
    selected
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["1", "2"]
  );
  assert!(
    select_fixed_episodes(&episodes, None, None)
      .unwrap()
      .is_none()
  );
}

#[test]
fn supports_multiple_episode_numbers_and_preserves_provider_order() {
  let episodes = vec![
    Episode::new("3", None, "three"),
    Episode::new("2", None, "two"),
    Episode::new("1", None, "one"),
  ];
  let selected = select_episode_range(episodes, "1 3").unwrap();
  assert_eq!(
    selected
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["3", "1"]
  );
}

#[test]
fn supports_first_last_and_numeric_episode_ranges() {
  let episodes = vec![
    Episode::new("1", None, "one"),
    Episode::new("1.5", None, "one and a half"),
    Episode::new("2", None, "two"),
    Episode::new("3", None, "three"),
  ];
  assert_eq!(
    select_episode_range(episodes.clone(), "0 -1")
      .unwrap()
      .iter()
      .map(|ep| ep.number.as_str())
      .collect::<Vec<_>>(),
    vec!["1", "3"]
  );
  assert_eq!(
    select_episode_range(episodes, "1-2")
      .unwrap()
      .iter()
      .map(|ep| ep.number.as_str())
      .collect::<Vec<_>>(),
    vec!["1", "1.5", "2"]
  );
}

#[test]
fn multi_episode_selection_is_sorted_and_deduplicated() {
  assert_eq!(
    ordered_episode_indices(vec![3, 1, 3, 0, 1, 2]),
    vec![0, 1, 2, 3]
  );
  assert!(ordered_episode_indices(Vec::new()).is_empty());
}

#[test]
fn numbered_fallback_selection_is_limited_to_the_current_page() {
  assert_eq!(parse_page_selection("16", 15, 30, false), Some(vec![15]));
  assert_eq!(
    parse_page_selection("16 18", 15, 30, true),
    Some(vec![15, 17])
  );
  assert_eq!(parse_page_selection("15", 15, 30, false), None);
  assert_eq!(parse_page_selection("31", 15, 30, false), None);
}

#[test]
fn auto_selects_only_single_result_lists() {
  assert_eq!(single_item_index(&[]), None);
  assert_eq!(
    single_item_index(&["Dragon Ball Super: Beerus".to_owned()]),
    Some(0)
  );
  assert_eq!(
    single_item_index(&["Frieren".to_owned(), "Blue Lock".to_owned()]),
    None
  );
}

#[test]
fn quality_menu_lists_best_worst_and_unique_resolutions() {
  let available = vec![
    "720p".to_owned(),
    "1080p".to_owned(),
    "720p".to_owned(),
    "360p".to_owned(),
  ];
  assert_eq!(
    quality_choices(&available),
    vec![
      ("best".to_owned(), "best".to_owned()),
      ("worst".to_owned(), "worst".to_owned()),
      ("1080p".to_owned(), "1080p".to_owned()),
      ("720p".to_owned(), "720p".to_owned()),
      ("360p".to_owned(), "360p".to_owned()),
    ]
  );
}

#[test]
fn finds_next_episode_after_last_watched_episode() {
  let episodes = vec![
    Episode::new("1", None, "one"),
    Episode::new("2", None, "two"),
    Episode::new("3", None, "three"),
  ];
  assert_eq!(
    next_episode_after(&episodes, "2").map(|episode| episode.number.as_str()),
    Some("3")
  );
}

#[test]
fn does_not_resume_finished_or_unmatched_history_entries() {
  let episodes = vec![
    Episode::new("1", None, "one"),
    Episode::new("2", None, "two"),
  ];
  assert_eq!(next_episode_after(&episodes, "2"), None);
  assert_eq!(next_episode_after(&episodes, "99"), None);
}

#[test]
fn normalizes_titles_for_exact_match() {
  assert_eq!(normalize_title("  BLUE   lock\n"), "blue lock");
}

#[test]
fn selects_multiple_episode_numbers_in_provider_order() {
  let episodes = vec![
    Episode::new("3", None, "three"),
    Episode::new("2", None, "two"),
    Episode::new("1", None, "one"),
  ];
  let selected = select_episode_range(episodes, "1 3").unwrap();
  assert_eq!(
    selected
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["3", "1"]
  );
}
