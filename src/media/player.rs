use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

#[cfg(unix)]
#[cfg(test)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

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

  pub fn supports_runtime_track_switching(&self) -> bool {
    cfg!(any(unix, windows))
      && matches!(
        self.kind(),
        PlayerKind::Mpv | PlayerKind::FlatpakMpv | PlayerKind::Iina
      )
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
        if let Some(subtitle) = subtitle {
          args.extend(["-e", "subtitles_location"].into_iter().map(OsString::from));
          args.push(OsString::from(&subtitle.url));
        }
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
    self.prepare_playback(stream, subtitle)?;
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
    self
      .launch_detached_internal(stream, subtitle, title, command_flags, media_flags, false)
      .map(|(child, _)| child)
  }

  pub fn launch_detached_with_track_control(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    command_flags: &[String],
    media_flags: &[String],
  ) -> io::Result<(Child, Option<MpvTrackController>)> {
    self.launch_detached_internal(stream, subtitle, title, command_flags, media_flags, true)
  }

  fn launch_detached_internal(
    &self,
    stream: &Stream,
    subtitle: Option<&Subtitle>,
    title: &str,
    command_flags: &[String],
    media_flags: &[String],
    enable_track_control: bool,
  ) -> io::Result<(Child, Option<MpvTrackController>)> {
    self.prepare_playback(stream, subtitle)?;
    let program = self.command_name();
    let mut args = self.arguments_with_options(stream, subtitle, title, command_flags, media_flags);
    let controller = if enable_track_control && self.supports_runtime_track_switching() {
      let (endpoint, address) = IpcEndpoint::new()?;
      let option = if self.kind() == PlayerKind::Iina {
        format!("--mpv-input-ipc-server={address}")
      } else {
        format!("--input-ipc-server={address}")
      };
      let stream_position = args
        .iter()
        .rposition(|argument| argument == &OsString::from(&stream.url))
        .unwrap_or(args.len());
      args.insert(stream_position, OsString::from(option));
      Some(MpvTrackController { endpoint })
    } else {
      None
    };
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null());
    if matches!(self, Self::AndroidMpv | Self::AndroidVlc) {
      command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
      command.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let child = command
      .spawn()
      .map_err(|error| contextualize_program_error(error, program))?;
    Ok((child, controller))
  }

  fn prepare_playback(&self, stream: &Stream, subtitle: Option<&Subtitle>) -> io::Result<()> {
    if matches!(self, Self::AndroidVlc) {
      if let Some(warning) = android_vlc_audio_warning(stream) {
        eprintln!("{warning}");
      }
      return Ok(());
    }
    if !matches!(self, Self::AndroidMpv) {
      return Ok(());
    }

    let config_path = std::path::Path::new("/storage/emulated/0/mpv");
    prepare_android_mpv_config(config_path, stream, subtitle).map_err(|error| {
      io::Error::new(
        error.kind(),
        format!(
          "could not prepare MPV-Android playback configuration in {}: {error}. Run `termux-setup-storage`, grant Termux shared-storage access, and ensure MPV-Android can read its shared mpv.conf",
          config_path.display()
        ),
      )
    })?;

    if stream.referer.is_some() {
      eprintln!(
        "watch: this stream requires MPV-Android to send a Referer header; verify that its shared mpv.conf includes mpv.config.mp4 and that MPV-Android can read the shared mpv directory"
      );
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct MpvTrackController {
  endpoint: IpcEndpoint,
}

impl MpvTrackController {
  pub fn set_audio_track(&self, url: Option<&str>) -> io::Result<()> {
    self.set_track("audio", url)
  }

  pub fn set_subtitle_track(&self, url: Option<&str>) -> io::Result<()> {
    self.set_track("sub", url)
  }

  fn set_track(&self, kind: &str, url: Option<&str>) -> io::Result<()> {
    let property = if kind == "audio" { "aid" } else { "sid" };
    let Some(url) = url else {
      self.request(json!(["set_property", property, "auto"]))?;
      return Ok(());
    };

    let tracks = self.request(json!(["get_property", "track-list"]))?;
    let existing_id = tracks
      .get("data")
      .and_then(Value::as_array)
      .and_then(|tracks| {
        tracks.iter().find_map(|track| {
          let matches_kind = track.get("type").and_then(Value::as_str) == Some(kind);
          let matches_url = track.get("external-filename").and_then(Value::as_str) == Some(url);
          (matches_kind && matches_url)
            .then(|| track.get("id").and_then(Value::as_u64))
            .flatten()
        })
      });

    if let Some(id) = existing_id {
      self.request(json!(["set_property", property, id]))?;
    } else if kind == "audio" {
      self.request(json!(["audio-add", url, "select"]))?;
    } else {
      self.request(json!(["sub-add", url, "select"]))?;
    }
    Ok(())
  }

  fn request(&self, command: Value) -> io::Result<Value> {
    let mut connection = BufReader::new(self.endpoint.connect()?);
    let request = json!({"command": command, "request_id": 1});
    let mut payload = serde_json::to_vec(&request).map_err(io::Error::other)?;
    payload.push(b'\n');
    connection.get_mut().write_all(&payload)?;
    connection.get_mut().flush()?;
    let mut response = String::new();
    connection.read_line(&mut response)?;
    let response: Value = serde_json::from_str(&response).map_err(io::Error::other)?;
    if response.get("error").and_then(Value::as_str) != Some("success") {
      return Err(io::Error::other(format!(
        "MPV IPC command failed: {}",
        response
          .get("error")
          .and_then(Value::as_str)
          .unwrap_or("invalid response")
      )));
    }
    Ok(response)
  }
}

#[derive(Debug)]
enum IpcEndpoint {
  #[cfg(unix)]
  Unix(std::path::PathBuf),
  #[cfg(windows)]
  Windows(String),
}

impl IpcEndpoint {
  fn new() -> io::Result<(Self, String)> {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_nanos();
    #[cfg(unix)]
    {
      let path = std::path::PathBuf::from(format!("/tmp/wmpv-{}-{nonce}.sock", std::process::id()));
      let address = path
        .to_str()
        .ok_or_else(|| io::Error::other("MPV IPC socket path is not valid UTF-8"))?
        .to_owned();
      Ok((Self::Unix(path), address))
    }
    #[cfg(windows)]
    {
      let address = format!(r"\\.\pipe\watch-mpv-{}-{nonce}", std::process::id());
      Ok((Self::Windows(address.clone()), address))
    }
    #[cfg(not(any(unix, windows)))]
    {
      let _ = nonce;
      Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "runtime MPV track switching is not supported on this platform",
      ))
    }
  }

  fn connect(&self) -> io::Result<Box<dyn IpcReadWrite>> {
    #[cfg(unix)]
    {
      let Self::Unix(path) = self;
      let stream = UnixStream::connect(path)?;
      stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
      Ok(Box::new(stream))
    }
    #[cfg(windows)]
    {
      let Self::Windows(path) = self;
      let stream = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
      Ok(Box::new(stream))
    }
    #[cfg(not(any(unix, windows)))]
    {
      Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "runtime MPV track switching is not supported on this platform",
      ))
    }
  }
}

trait IpcReadWrite: io::Read + io::Write {}

impl<T: io::Read + io::Write> IpcReadWrite for T {}

#[cfg(unix)]
impl Drop for IpcEndpoint {
  fn drop(&mut self) {
    let Self::Unix(path) = self;
    let _ = std::fs::remove_file(path);
  }
}

fn prepare_android_mpv_config(
  config_dir: &std::path::Path,
  stream: &Stream,
  subtitle: Option<&Subtitle>,
) -> io::Result<()> {
  ensure_android_mpv_config(config_dir, &android_mpv_config(stream, subtitle))
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

fn android_vlc_audio_warning(stream: &Stream) -> Option<&'static str> {
  stream.audio_url.as_ref().map(|_| {
    "watch: warning: VLC Android's external VIEW intent does not support a separate audio URL; the selected external audio track cannot be applied"
  })
}
