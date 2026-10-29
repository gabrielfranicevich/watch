use super::*;

fn stream() -> Stream {
  Stream::new(
    "https://cdn.example/video.m3u8",
    Some("1080p".into()),
    Some("https://embed.example/".into()),
  )
}

#[test]
fn mpv_arguments_preserve_stream_referer_and_subtitle() {
  let subtitle = Subtitle::new("https://cdn.example/sub.vtt", Some("English".into()));
  let args = Player::Mpv.arguments(&stream(), Some(&subtitle), "Example Anime");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(args.contains(&"--referrer=https://embed.example/".to_owned()));
  assert!(args.contains(&"--sub-file=https://cdn.example/sub.vtt".to_owned()));
  assert!(args.contains(&"--force-media-title=Example Anime".to_owned()));
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn vlc_arguments_include_referer_and_exit_after_play() {
  let args = Player::Vlc.arguments(&stream(), None, "Example Anime");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(args.contains(&"--http-referrer=https://embed.example/".to_owned()));
  assert!(args.contains(&"--play-and-exit".to_owned()));
  assert!(args.contains(&"--meta-title=Example Anime".to_owned()));
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn appends_extra_flags_before_the_media_url() {
  let args = Player::Mpv.arguments_with_extra(
    &stream(),
    None,
    "Example Anime",
    &["--no-terminal".to_owned()],
  );
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(args.contains(&"--no-terminal".to_owned()));
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn vlc_uses_input_slave_for_subtitle_without_mpv_sub_file_flag() {
  let subtitle = Subtitle::new("https://cdn.example/sub.vtt", Some("English".into()));
  let args = Player::Vlc.arguments(&stream(), Some(&subtitle), "Example");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();

  assert!(!args.iter().any(|arg| arg.starts_with("--sub-file=")));
  assert!(
    args
      .iter()
      .any(|arg| arg == ":input-slave=https://cdn.example/sub.vtt")
  );
}

#[test]
fn forwards_external_audio_to_mpv_and_vlc() {
  let stream = stream().with_audio(Some("https://cdn.example/audio.m3u8".into()));
  let mpv = Player::Mpv.arguments(&stream, None, "Example");
  assert!(
    mpv
      .iter()
      .any(|arg| arg == "--audio-file=https://cdn.example/audio.m3u8")
  );

  let vlc = Player::Vlc.arguments(&stream, None, "Example");
  assert!(
    vlc
      .iter()
      .any(|arg| arg == ":input-slave=https://cdn.example/audio.m3u8")
  );
}

#[test]
fn syncplay_passes_media_url_and_mpv_arguments_after_separator() {
  let stream = stream().with_audio(Some("https://cdn.example/audio.m3u8".into()));
  let args = Player::Syncplay.arguments_with_options(
    &stream,
    None,
    "Example Anime",
    &["--player-flag".to_owned()],
    &["--start=1".to_owned()],
  );
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  let url_index = args
    .iter()
    .position(|arg| arg == "https://cdn.example/video.m3u8")
    .unwrap();
  assert_eq!(args[url_index - 1], "--player-flag");
  assert_eq!(args[url_index + 1], "--");
  assert!(args.contains(&"--audio-file=https://cdn.example/audio.m3u8".to_owned()));
  assert!(args.contains(&"--start=1".to_owned()));
}

#[test]
fn iina_arguments_forward_mpv_media_options_and_escape_colons() {
  let stream = stream().with_audio(Some("https://cdn.example/audio.m3u8".into()));
  let subtitle = Subtitle::new("https://cdn.example/sub.vtt", Some("English".into()));
  let args = Player::Iina.arguments(&stream, Some(&subtitle), "Example Anime");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(args.contains(&"--mpv-referrer=https://embed.example/".to_owned()));
  assert!(args.contains(&"--mpv-audio-files=https\\://cdn.example/audio.m3u8".to_owned()));
  assert!(args.contains(&"--mpv-sub-files=https\\://cdn.example/sub.vtt".to_owned()));
  assert!(args.contains(&"--no-stdin".to_owned()));
  assert!(args.contains(&"--keep-running".to_owned()));
  assert!(args.contains(&"--mpv-force-media-title=Example Anime".to_owned()));
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn android_mpv_uses_termux_open_with_video_mime_and_chooser() {
  let args = Player::AndroidMpv.arguments(&stream(), None, "Frieren Episode 4");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert_eq!(
    args,
    vec![
      "--chooser",
      "--content-type",
      "video/*",
      "https://cdn.example/video.m3u8"
    ]
  );
  assert_eq!(Player::AndroidMpv.command_name(), "termux-open");
}

#[test]
fn flatpak_mpv_supports_intro_skip_options() {
  assert!(Player::FlatpakMpv.supports_intro_skipping());
}

#[test]
fn android_vlc_uses_the_vlc_activity_and_title_extra() {
  let args = Player::AndroidVlc.arguments(&stream(), None, "Frieren Episode 4");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert!(args.windows(2).any(|pair| {
    pair[0] == "-n" && pair[1] == "org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity"
  }));
  assert!(args.contains(&"Frieren Episode 4".to_owned()));
  assert_eq!(Player::AndroidVlc.command_name(), "am");
}

#[test]
fn android_mpv_config_contains_referer_audio_and_subtitle_lines() {
  let stream = stream().with_audio(Some("https://cdn.example/audio.m3u8".into()));
  let subtitle = Subtitle::new("https://cdn.example/sub.vtt", None);
  assert_eq!(
    android_mpv_config(&stream, Some(&subtitle)),
    concat!(
      "referrer=https://embed.example/\n",
      "sub-file=https://cdn.example/sub.vtt\n",
      "audio-file=https://cdn.example/audio.m3u8\n"
    )
  );
}

#[test]
fn android_mpv_config_is_included_by_the_standard_config_and_preserves_user_settings() {
  let config_dir = std::env::temp_dir().join(format!(
    "watch-mpv-config-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  std::fs::create_dir_all(&config_dir).unwrap();
  std::fs::write(config_dir.join("mpv.conf"), "hwdec=auto").unwrap();

  ensure_android_mpv_config(&config_dir, "referrer=https://embed.example/\n").unwrap();
  ensure_android_mpv_config(&config_dir, "referrer=https://embed.example/\n").unwrap();

  let main_config = std::fs::read_to_string(config_dir.join("mpv.conf")).unwrap();
  assert!(main_config.starts_with("hwdec=auto\n"));
  let include_line = format!("include={}", config_dir.join("mpv.config.mp4").display());
  assert_eq!(
    main_config
      .lines()
      .filter(|line| line.trim() == include_line.as_str())
      .count(),
    1
  );
  assert_eq!(
    std::fs::read_to_string(config_dir.join("mpv.config.mp4")).unwrap(),
    "referrer=https://embed.example/\n"
  );

  std::fs::remove_dir_all(config_dir).unwrap();
}

#[test]
fn flatpak_mpv_uses_the_flatpak_application_entrypoint() {
  let args = Player::FlatpakMpv.arguments(&stream(), None, "Example");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert_eq!(args[0], "run");
  assert_eq!(args[1], "io.mpv.Mpv");
  assert!(args.contains(&"--referrer=https://embed.example/".to_owned()));
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn command_names_follow_host_platform_for_standard_players() {
  if cfg!(target_os = "windows") {
    assert_eq!(Player::Mpv.command_name(), "mpv.com");
    assert_eq!(Player::Vlc.command_name(), "vlc.exe");
    assert_eq!(Player::Syncplay.command_name(), "syncplay.exe");
  }
  if cfg!(target_os = "macos") {
    assert_eq!(
      Player::Syncplay.command_name(),
      "/Applications/Syncplay.app/Contents/MacOS/syncplay"
    );
  }
}

#[test]
fn configured_player_uses_mpv_specific_arguments_when_named_mpv() {
  let player = Player::from_configuration(false, false, false, Some("/opt/mpv/mpv.exe"));
  assert_eq!(player.command_name(), "/opt/mpv/mpv.exe");
  let args = player.arguments(&stream(), None, "Example");
  assert!(
    args
      .iter()
      .any(|arg| arg == "--referrer=https://embed.example/")
  );
  assert!(args.iter().any(|arg| arg == "--force-media-title=Example"));
}

#[test]
fn configured_flatpak_mpv_path_runs_the_flatpak_app() {
  let configured =
    "/home/user/.local/share/flatpak/app/io.mpv.Mpv/x86_64/stable/active/files/bin/mpv";
  let player = Player::from_configuration(false, false, false, Some(configured));
  assert_eq!(player.command_name(), "flatpak");
  assert!(player.supports_intro_skipping());

  let args = player.arguments(&stream(), None, "Example");
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert_eq!(args[0], "run");
  assert_eq!(args[1], "io.mpv.Mpv");
  assert_eq!(
    args.last().map(String::as_str),
    Some("https://cdn.example/video.m3u8")
  );
}

#[test]
fn configured_catt_player_uses_cast_subcommand() {
  let player = Player::from_configuration(false, false, false, Some("catt"));
  let args = player.arguments_with_extra(
    &stream(),
    None,
    "Example",
    &["--device".to_owned(), "Living Room".to_owned()],
  );
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert_eq!(
    args,
    vec![
      "cast",
      "--device",
      "Living Room",
      "https://cdn.example/video.m3u8"
    ]
  );
}

#[test]
fn configured_custom_player_receives_generic_flags_and_stream_url() {
  let player = Player::from_configuration(false, false, false, Some("/usr/bin/custom-player"));
  let args = player.arguments_with_extra(&stream(), None, "Example", &["--custom-flag".to_owned()]);
  let args: Vec<_> = args
    .iter()
    .map(|arg| arg.to_string_lossy().into_owned())
    .collect();
  assert_eq!(
    args,
    vec!["--custom-flag", "https://cdn.example/video.m3u8"]
  );
}

#[test]
fn missing_playback_program_error_names_the_executable_and_configuration_option() {
  let error = contextualize_program_error(
    io::Error::new(io::ErrorKind::NotFound, "program not found"),
    "mpv.exe",
  );
  assert_eq!(error.kind(), io::ErrorKind::NotFound);
  assert!(error.to_string().contains("mpv.exe"));
  assert!(error.to_string().contains("ANI_CLI_PLAYER"));
}

#[test]
fn only_mpv_variants_support_terminal_video_output() {
  assert!(Player::Mpv.supports_terminal_video());
  assert!(Player::FlatpakMpv.supports_terminal_video());
  assert!(Player::Configured("/opt/mpv".to_owned()).supports_terminal_video());
  assert!(!Player::Vlc.supports_terminal_video());
  assert!(!Player::Syncplay.supports_terminal_video());
}

#[test]
fn selects_iina_when_requested_without_other_player_flags() {
  assert_eq!(Player::from_options(false, false, true), Player::Iina);
  assert_eq!(Player::from_options(true, false, true), Player::Vlc);
  assert_eq!(Player::from_options(false, true, true), Player::Syncplay);
}

#[test]
fn chooses_syncplay_when_requested() {
  assert_eq!(Player::from_flags(false, true), Player::Syncplay);
}

#[test]
fn selects_mpv_by_default_and_vlc_when_requested() {
  assert_eq!(Player::from_vlc_flag(false), Player::Mpv);
  assert_eq!(Player::from_vlc_flag(true), Player::Vlc);
}


#[test]
fn android_mpv_config_can_be_cleared_without_leaving_stale_stream_options() {
  let config_dir = std::env::temp_dir().join(format!(
    "watch-mpv-empty-config-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  std::fs::create_dir_all(&config_dir).unwrap();

  ensure_android_mpv_config(&config_dir, "referrer=https://old.example/\\nsub-file=https://old.example/sub.vtt\\n").unwrap();
  ensure_android_mpv_config(&config_dir, "").unwrap();

  assert_eq!(
    std::fs::read_to_string(config_dir.join("mpv.config.mp4")).unwrap(),
    ""
  );
  let main_config = std::fs::read_to_string(config_dir.join("mpv.conf")).unwrap();
  let include_line = format!("include={}", config_dir.join("mpv.config.mp4").display());
  assert_eq!(
    main_config.lines().filter(|line| *line == include_line).count(),
    1
  );

  std::fs::remove_dir_all(config_dir).unwrap();
}

#[test]
fn android_mpv_config_reports_filesystem_errors() {
  let root = std::env::temp_dir().join(format!(
    "watch-mpv-config-error-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ));
  std::fs::write(&root, "not a directory").unwrap();

  let error = ensure_android_mpv_config(&root, "referrer=https://embed.example/\\n").unwrap_err();
  assert!(matches!(
    error.kind(),
    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory
  ));

  std::fs::remove_file(root).unwrap();
}
