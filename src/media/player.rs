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
      Self::AndroidMpv => "termux-open",
      Self::AndroidVlc => "am",
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
        // Route the VIEW intent through Termux's supported external-app
        // receiver. Direct `am start` is rejected by some Android firmware
        // builds (including Unisoc devices) before MPV can receive the URL.
        // MPV-specific stream options are written to its shared config by
        // prepare_playback(), since termux-open cannot pass arbitrary MPV flags.
        args.extend(
          ["--chooser", "--content-type", "video/*"]
            .into_iter()
            .map(OsString::from),
        );
        args.push(OsString::from(&stream.url));
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

    if stream.referer.is_some() {
      eprintln!(
        "watch: this stream requires MPV-Android to send a Referer header.          In Android Settings, grant MPV-Android (is.xyz.mpv)          \"Allow access to manage all files\"; otherwise the player may open and immediately stop."
      );
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
  let existing = match std::fs::read_to_string(&main_config) {
    Ok(content) => content,
    Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
    Err(error) => return Err(error),
  };
  if existing
    .lines()
    .any(|line| line.trim() == include_line.as_str())
  {
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
#[path = "../../tests/unit/media/player.rs"]
mod tests;
