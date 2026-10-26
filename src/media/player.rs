use std::ffi::OsString;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};

use crate::domain::{stream::Stream, subtitle::Subtitle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Mpv,
    Vlc,
    Syncplay,
}

impl Player {
    pub fn from_vlc_flag(use_vlc: bool) -> Self {
        Self::from_flags(use_vlc, false)
    }

    pub fn from_flags(use_vlc: bool, use_syncplay: bool) -> Self {
        if use_syncplay {
            Self::Syncplay
        } else if use_vlc {
            Self::Vlc
        } else {
            Self::Mpv
        }
    }

    pub fn command_name(self) -> &'static str {
        match self {
            Self::Mpv => "mpv",
            Self::Vlc => "vlc",
            Self::Syncplay => "syncplay",
        }
    }

    pub fn arguments(
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
    ) -> Vec<OsString> {
        self.arguments_with_options(stream, subtitle, title, &[], &[])
    }

    pub fn arguments_with_extra(
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
        extra_flags: &[String],
    ) -> Vec<OsString> {
        self.arguments_with_options(stream, subtitle, title, extra_flags, &[])
    }

    pub fn arguments_with_options(
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
        command_flags: &[String],
        media_flags: &[String],
    ) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Self::Mpv => {
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
            Self::Vlc => {
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
            Self::Syncplay => {
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
        }
        args
    }

    pub fn play(
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
    ) -> io::Result<ExitStatus> {
        self.play_with_extra(stream, subtitle, title, &[])
    }

    pub fn play_with_extra(
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
        extra_flags: &[String],
    ) -> io::Result<ExitStatus> {
        self.play_with_options(stream, subtitle, title, extra_flags, &[])
    }

    pub fn play_with_options(
        self,
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
        self,
        stream: &Stream,
        subtitle: Option<&Subtitle>,
        title: &str,
        extra_flags: &[String],
    ) -> io::Result<Child> {
        self.launch_detached_with_options(stream, subtitle, title, extra_flags, &[])
    }

    pub fn launch_detached_with_options(
        self,
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
    fn chooses_syncplay_when_requested() {
        assert_eq!(Player::from_flags(false, true), Player::Syncplay);
    }

    #[test]
    fn selects_mpv_by_default_and_vlc_when_requested() {
        assert_eq!(Player::from_vlc_flag(false), Player::Mpv);
        assert_eq!(Player::from_vlc_flag(true), Player::Vlc);
    }
}
