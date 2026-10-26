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
    Configured(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    Mpv,
    Vlc,
    Syncplay,
    Iina,
    FlatpakMpv,
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
            Self::Vlc
        } else if let Some(command) = configured_player.filter(|value| !value.trim().is_empty()) {
            Self::Configured(command.to_owned())
        } else if use_iina_default {
            Self::Iina
        } else {
            Self::Mpv
        }
    }

    pub fn command_name(&self) -> &str {
        match self {
            Self::Mpv if cfg!(target_os = "windows") => "mpv.exe",
            Self::Mpv => "mpv",
            Self::Vlc if cfg!(target_os = "windows") => "vlc.exe",
            Self::Vlc => "vlc",
            Self::Syncplay if cfg!(target_os = "macos") => {
                "/Applications/Syncplay.app/Contents/MacOS/syncplay"
            }
            Self::Syncplay if cfg!(target_os = "windows") => "syncplay.exe",
            Self::Syncplay => "syncplay",
            Self::Iina if cfg!(target_os = "macos") => {
                "/Applications/IINA.app/Contents/MacOS/iina-cli"
            }
            Self::Iina => "iina-cli",
            Self::FlatpakMpv => "flatpak",
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
            Self::Configured(command) => {
                let name = std::path::Path::new(command)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(command)
                    .to_ascii_lowercase();
                if name.contains("syncplay") {
                    PlayerKind::Syncplay
                } else if name.contains("vlc") {
                    PlayerKind::Vlc
                } else if name.contains("iina") {
                    PlayerKind::Iina
                } else if name.contains("mpv") {
                    PlayerKind::Mpv
                } else {
                    PlayerKind::Custom
                }
            }
        }
    }

    pub fn supports_intro_skipping(&self) -> bool {
        matches!(self.kind(), PlayerKind::Mpv | PlayerKind::Syncplay)
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
                if let Some(subtitle) = subtitle {
                    args.push(OsString::from(format!("--sub-file={}", subtitle.url)));
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
        Command::new(self.command_name())
            .args(self.arguments_with_options(stream, subtitle, title, command_flags, media_flags))
            .status()
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
        Command::new(self.command_name())
            .args(self.arguments_with_options(stream, subtitle, title, command_flags, media_flags))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
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
    fn forwards_external_audio_to_mpv_and_vlc() {
        let stream = stream().with_audio(Some("https://cdn.example/audio.m3u8".into()));
        let mpv = Player::Mpv.arguments(&stream, None, "Example");
        assert!(
            mpv.iter()
                .any(|arg| arg == "--audio-file=https://cdn.example/audio.m3u8")
        );

        let vlc = Player::Vlc.arguments(&stream, None, "Example");
        assert!(
            vlc.iter()
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
            assert_eq!(Player::Mpv.command_name(), "mpv.exe");
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
            args.iter()
                .any(|arg| arg == "--referrer=https://embed.example/")
        );
        assert!(args.iter().any(|arg| arg == "--force-media-title=Example"));
    }

    #[test]
    fn configured_custom_player_receives_generic_flags_and_stream_url() {
        let player =
            Player::from_configuration(false, false, false, Some("/usr/bin/custom-player"));
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
