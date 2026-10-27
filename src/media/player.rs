use std::ffi::OsString;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::domain::{stream::Stream, subtitle::Subtitle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Player {
  Mpv,
  Vlc,
  Syncplay,
  Iina,
  FlatpakMpv,
  AndroidMpv,
  AndroidVlc,
  Configured(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
  Mpv,
  Vlc,
  Syncplay,
  Iina,
  FlatpakMpv,
  AndroidMpv,
  AndroidVlc,
  Catt,
  Custom,
}

impl Player {
  pub fn from_vlc_flag(use_vlc: bool) -> Self {
    Self::from_flags(use_vlc, false)
  }

  pub fn from_flags(use_vlc: bool, use_syncplay: bool) -> Self {
    Self::from_options(use_vlc, use_syncplay, false)
  }

  pub fn from_options(use_vlc: bool, use_syncplay: bool, use_iina: bool) -> Self {
    if use_syncplay {
      Self::Syncplay
    } else if use_vlc {
      Self::Vlc
    } else if use_iina {
      Self::Iina
    } else {
      Self::Mpv
    }
  }

  pub fn from_configuration(
    use_vlc: bool,
    use_syncplay: bool,
    use_iina_default: bool,
    configured_player: Option<&str>,
  ) -> Self {
    if use_syncplay {
      Self::Syncplay
    } else if use_vlc {
      if cfg!(target_os = "android") {
        Self::AndroidVlc
      } else {
        Self::Vlc
      }
    } else if let Some(command) = configured_player.filter(|value| !value.trim().is_empty()) {
      if cfg!(target_os = "android") && command.to_ascii_lowercase().contains("vlc") {
        Self::AndroidVlc
      } else if cfg!(target_os = "android") && command.to_ascii_lowercase().contains("mpv") {
        Self::AndroidMpv
      } else {
        Self::Configured(command.to_owned())
      }
    } else if cfg!(target_os = "android") {
      Self::AndroidMpv
    } else if use_iina_default {
      Self::Iina
    } else {
      Self::Mpv
    }
  }

  pub fn command_name(&self) -> &str {
    match self {
      // The .com launcher keeps MPV attached to the Windows console so
      // terminal video outputs can write graphics data to stdout.
      Self::Mpv if cfg!(target_os = "windows") => "mpv.com",
      Self::Mpv => "mpv",
      Self::Vlc if cfg!(target_os = "windows") => "vlc.exe",
      Self::Vlc => "vlc",
      Self::Syncplay if cfg!(target_os = "macos") => {
        "/Applications/Syncplay.app/Contents/MacOS/syncplay"
      }
      Self::Syncplay if cfg!(target_os = "windows") => "syncplay.exe",
      Self::Syncplay => "syncplay",
      Self::Iina if cfg!(target_os = "macos") => "/Applications/IINA.app/Contents/MacOS/iina-cli",
      Self::Iina => "iina-cli",
      Self::FlatpakMpv => "flatpak",
      Self::AndroidMpv | Self::AndroidVlc => "am",
      Self::Configured(_) if self.kind() == PlayerKind::FlatpakMpv => "flatpak",
      Self::Configured(command) => command,
    }
  }

  fn kind(&self) -> PlayerKind {
    match self {
      Self::Mpv => PlayerKind::Mpv,
      Self::Vlc => PlayerKind::Vlc,
      Self::Syncplay => PlayerKind::Syncplay,
      Self::Iina => PlayerKind::Iina,
      Self::FlatpakMpv => PlayerKind::FlatpakMpv,
      Self::AndroidMpv => PlayerKind::AndroidMpv,
      Self::AndroidVlc => PlayerKind::AndroidVlc,
      Self::Configured(command) => {
        let command_lower = command.to_ascii_lowercase();
        let name = std::path::Path::new(command)
          .file_name()
          .and_then(|value| value.to_str())
          .unwrap_or(command)
          .to_ascii_lowercase();
        if command_lower.contains("flatpak") && command_lower.contains("mpv") {
          PlayerKind::FlatpakMpv
        } else if name.contains("syncplay") {
          PlayerKind::Syncplay
        } else if name.contains("vlc") {
          PlayerKind::Vlc
        } else if name.contains("iina") {
          PlayerKind::Iina
        } else if name.contains("mpv") {
          PlayerKind::Mpv
        } else if name.contains("catt") {
          PlayerKind::Catt
        } else {
          PlayerKind::Custom
        }
      }
    }
  }

  pub fn supports_intro_skipping(&self) -> bool {
    matches!(
      self.kind(),
      PlayerKind::Mpv | PlayerKind::FlatpakMpv | PlayerKind::Syncplay
    )
  }

  pub fn supports_terminal_video(&self) -> bool {
    matches!(self.kind(), PlayerKind::Mpv | PlayerKind::FlatpakMpv)
  }

  pub fn terminal_video_output_help(&self) -> io::Result<String> {
    let program = self.command_name();
    let mut command = Command::new(program);
    if self.kind() == PlayerKind::FlatpakMpv {
      command.args(["run", "io.mpv.Mpv"]);
    }
    let output = command
      .arg("--vo=help")
      .output()
      .map_err(|error| contextualize_program_error(error, program))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{stdout}\n{stderr}");
    if output.status.success() {
      Ok(text)
    } else {
      Err(io::Error::other(format!(
        "{program} --vo=help failed: {}",
        text.trim()
      )))
    }
  }

  pub fn arguments(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
  ) -> Vec<OsString> {
    self.arguments_with_options(stream, subtitle, title, &[], &[])
  }

  pub fn arguments_with_extra(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    extra_flags: &[String],
  ) -> Vec<OsString> {
    self.arguments_with_options(stream, subtitle, title, extra_flags, &[])
  }

  pub fn arguments_with_options(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    command_flags: &[String],
    media_flags: &[String],
  ) -> Vec<OsString> {
    let mut args = Vec::new();
    match self.kind() {
      PlayerKind::Mpv | PlayerKind::FlatpakMpv => {
        if matches!(self.kind(), PlayerKind::FlatpakMpv) {
          args.push(OsString::from("run"));
          args.push(OsString::from("io.mpv.Mpv"));
        }
        args.push(OsString::from("--force-window"));
        args.push(OsString::from(format!("--force-media-title={title}")));
        if let Some(referer) = stream.referer.as_deref() {
          args.push(OsString::from(format!("--referrer={referer}")));
        }
        if let Some(audio) = stream.audio_url.as_deref() {
          args.push(OsString::from(format!("--audio-file={audio}")));
        }
        if let Some(subtitle) = subtitle {
          args.push(OsString::from(format!("--sub-file={}", subtitle.url)));
        }
        args.extend(command_flags.iter().map(OsString::from));
        args.extend(media_flags.iter().map(OsString::from));
        args.push(OsString::from(&stream.url));
      }
      PlayerKind::Vlc => {
        args.push(OsString::from("--play-and-exit"));
        args.push(OsString::from(format!("--meta-title={title}")));
        if let Some(referer) = stream.referer.as_deref() {
          args.push(OsString::from(format!("--http-referrer={referer}")));
        }
        args.extend(command_flags.iter().map(OsString::from));
        args.push(OsString::from(&stream.url));

        let mut slave_inputs = Vec::new();
        if let Some(audio) = stream.audio_url.as_deref() {
          slave_inputs.push(audio);
        }
        if let Some(subtitle) = subtitle {
          slave_inputs.push(subtitle.url.as_str());
        }
        if !slave_inputs.is_empty() {
          args.push(OsString::from(format!(
            ":input-slave={}",
            slave_inputs.join("#")
          )));
        }
      }
      PlayerKind::Iina => {
        if let Some(referer) = stream.referer.as_deref() {
          args.push(OsString::from(format!("--mpv-referrer={referer}")));
        }
        if let Some(audio) = stream.audio_url.as_deref() {
          args.push(OsString::from(format!(
            "--mpv-audio-files={}",
            escape_iina_url(audio)
          )));
        }
        if let Some(subtitle) = subtitle {
          args.push(OsString::from(format!(
            "--mpv-sub-files={}",
            escape_iina_url(&subtitle.url)
          )));
        }
        args.extend(command_flags.iter().map(OsString::from));
        args.push(OsString::from("--no-stdin"));
        args.push(OsString::from("--keep-running"));
        args.push(OsString::from(format!("--mpv-force-media-title={title}")));
        args.push(OsString::from(&stream.url));
      }
      PlayerKind::Syncplay => {
        args.extend(command_flags.iter().map(OsString::from));
        args.push(OsString::from(&stream.url));
        args.push(OsString::from("--"));
        if let Some(referer) = stream.referer.as_deref() {
          args.push(OsString::from(format!("--referrer={referer}")));
        }
        if let Some(audio) = stream.audio_url.as_deref() {
          args.push(OsString::from(format!("--audio-file={audio}")));
        }
        if let Some(subtitle) = subtitle {
          args.push(OsString::from(format!("--sub-file={}", subtitle.url)));
        }
        args.push(OsString::from(format!("--force-media-title={title}")));
        args.extend(media_flags.iter().map(OsString::from));
      }
      PlayerKind::AndroidMpv => {
        // Follow mpv-android's documented external intent interface: let
        // Android resolve the exported VIEW activity by package rather than
        // hard-coding an Activity class that can change between app builds.
        args.extend(
          [
            "start",
            "--user",
            "0",
            "-a",
            "android.intent.action.VIEW",
            "-t",
            "video/any",
            "-p",
            "is.xyz.mpv",
            "-d",
          ]
          .into_iter()
          .map(OsString::from),
        );
        args.push(OsString::from(&stream.url));
        args.extend(["-e", "title"].into_iter().map(OsString::from));
        args.push(OsString::from(title));
        args.extend(command_flags.iter().map(OsString::from));
      }
      PlayerKind::AndroidVlc => {
        args.extend(
          [
            "start",
            "--user",
            "0",
            "-a",
            "android.intent.action.VIEW",
            "-d",
          ]
          .into_iter()
          .map(OsString::from),
        );
        args.push(OsString::from(&stream.url));
        args.extend(
          [
            "-n",
            "org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity",
            "-e",
            "title",
          ]
          .into_iter()
          .map(OsString::from),
        );
        args.push(OsString::from(title));
        args.extend(command_flags.iter().map(OsString::from));
      }
      PlayerKind::Catt => {
        args.push(OsString::from("cast"));
        args.extend(command_flags.iter().map(OsString::from));
        args.push(OsString::from(&stream.url));
      }
      PlayerKind::Custom => {
        args.extend(command_flags.iter().map(OsString::from));
        args.push(OsString::from(&stream.url));
      }
    }
    args
  }

  pub fn play(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
  ) -> io::Result<ExitStatus> {
    self.play_with_extra(stream, subtitle, title, &[])
  }

  pub fn play_with_extra(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    extra_flags: &[String],
  ) -> io::Result<ExitStatus> {
    self.play_with_options(stream, subtitle, title, extra_flags, &[])
  }

  pub fn play_with_options(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    command_flags: &[String],
    media_flags: &[String],
  ) -> io::Result<ExitStatus> {
    self.prepare_playback(stream, subtitle);
    let program = self.command_name();
    Command::new(program)
      .args(self.arguments_with_options(stream, subtitle, title, command_flags, media_flags))
      .status()
      .map_err(|error| contextualize_program_error(error, program))
  }

  /// Start the player in the background, leaving the terminal available for
  /// ani-cli's episode controls.
  pub fn launch_detached(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    extra_flags: &[String],
  ) -> io::Result<Child> {
    self.launch_detached_with_options(stream, subtitle, title, extra_flags, &[])
  }

  pub fn launch_detached_with_options(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    command_flags: &[String],
    media_flags: &[String],
  ) -> io::Result<Child> {
    self.prepare_playback(stream, subtitle);
    let program = self.command_name();
    let mut command = Command::new(program);
    command
      .args(self.arguments_with_options(stream, subtitle, title, command_flags, media_flags))
      .stdin(Stdio::null());
    if matches!(self, Self::AndroidMpv | Self::AndroidVlc) {
      // Keep Android Activity Manager errors visible; the detached child is
      // only the short-lived `am` command, not the media player process.
      command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
      command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    command
      .spawn()
      .map_err(|error| contextualize_program_error(error, program))
  }
  fn prepare_playback(&self, stream: &Stream, subtitle: Option<&Subtitle>) {
    if !matches!(self, Self::AndroidMpv) {
      return;
    }

    let config = android_mpv_config(stream, subtitle);
    if config.is_empty() {
      return;
    }

    let config_path = std::path::Path::new("/storage/emulated/0/mpv");
    if let Err(error) = ensure_android_mpv_config(config_path, &config) {
      eprintln!(
        "watch: warning: could not configure MPV for this stream: {error}. \
         Run `termux-setup-storage` and allow MPV-Android access to shared storage; \
         streams requiring a Referer header may otherwise fail."
      );
    }
  }
}

fn ensure_android_mpv_config(config_dir: &std::path::Path, config: &str) -> io::Result<()> {
  std::fs::create_dir_all(config_dir)?;

  let stream_config = config_dir.join("mpv.config.mp4");
  std::fs::write(&stream_config, config)?;

  // MPV-Android does not automatically load arbitrary files in shared storage.
  // Ensure its standard shared-storage config includes the per-stream options.
  let main_config = config_dir.join("mpv.conf");
  let include_line = format!("include={}", stream_config.display());
  let existing = std::fs::read_to_string(&main_config).unwrap_or_default();
  if existing.lines().any(|line| line.trim() == include_line.as_str()) {
    return Ok(());
  }

  use std::io::Write as _;
  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(main_config)?;
  if !existing.is_empty() && !existing.ends_with('\n') {
    file.write_all(b"\n")?;
  }
  writeln!(file, "{include_line}")?;
  Ok(())
}

fn contextualize_program_error(error: io::Error, program: &str) -> io::Error {
  if error.kind() == io::ErrorKind::NotFound {
    io::Error::new(
      error.kind(),
      format!(
        "playback program `{program}` was not found; install it and ensure it is on PATH, or set ANI_CLI_PLAYER to its executable path"
      ),
    )
  } else {
    error
  }
}

fn android_mpv_config(stream: &Stream, subtitle: Option<&Subtitle>) -> String {
  let mut lines = Vec::new();
  if let Some(referer) = stream.referer.as_deref() {
    lines.push(format!("referrer={referer}"));
  }
  if let Some(subtitle) = subtitle {
    lines.push(format!("sub-file={}", subtitle.url));
  }
  if let Some(audio) = stream.audio_url.as_deref() {
    lines.push(format!("audio-file={audio}"));
  }
  if lines.is_empty() {
    String::new()
  } else {
    format!("{}\n", lines.join("\n"))
  }
}

fn escape_iina_url(url: &str) -> String {
  url.replace(':', "\\:")
}

#[cfg(test)]
mod tests {
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
  fn android_mpv_uses_the_mpv_activity_and_title_extra() {
    let args = Player::AndroidMpv.arguments_with_extra(
      &stream(),
      None,
      "Frieren Episode 4",
      &["--debug".to_owned()],
    );
    let args: Vec<_> = args
      .iter()
      .map(|arg| arg.to_string_lossy().into_owned())
      .collect();
    assert_eq!(args[0], "start");
    assert_eq!(args[1], "--user");
    assert!(
      args
        .windows(2)
        .any(|pair| pair[0] == "-t" && pair[1] == "video/any")
    );
    assert!(
      args
        .windows(2)
        .any(|pair| pair[0] == "-p" && pair[1] == "is.xyz.mpv")
    );
    assert!(!args.iter().any(|arg| arg == "-n"));
    assert!(
      args
        .windows(2)
        .any(|pair| pair[0] == "-e" && pair[1] == "title")
    );
    assert!(args.contains(&"Frieren Episode 4".to_owned()));
    assert!(args.contains(&"--debug".to_owned()));
    assert_eq!(Player::AndroidMpv.command_name(), "am");
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
      pair[0] == "-n"
        && pair[1] == "org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity"
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
    let include_line = format!("include={}/mpv.config.mp4", config_dir.display());
    assert_eq!(
      main_config.lines().filter(|line| line.trim() == include_line.as_str()).count(),
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
    let args =
      player.arguments_with_extra(&stream(), None, "Example", &["--custom-flag".to_owned()]);
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
}
