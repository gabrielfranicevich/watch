use std::ffi::OsString;
use std::io;
use std::process::{Command, ExitStatus};

use crate::domain::{stream::Stream, subtitle::Subtitle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Mpv,
    Vlc,
}

impl Player {
    pub fn from_vlc_flag(use_vlc: bool) -> Self {
        if use_vlc { Self::Vlc } else { Self::Mpv }
    }

    pub fn command_name(self) -> &'static str {
        match self {
            Self::Mpv => "mpv",
            Self::Vlc => "vlc",
        }
    }

    pub fn arguments(self, stream: &Stream, subtitle: Option<&Subtitle>, title: &str) -> Vec<OsString> {
        let mut args = Vec::new();
        match self {
            Self::Mpv => {
                args.push(OsString::from("--force-window"));
                args.push(OsString::from(format!("--title={title}")));
                if let Some(referer) = stream.referer.as_deref() {
                    args.push(OsString::from(format!("--referrer={referer}")));
                }
                if let Some(subtitle) = subtitle {
                    args.push(OsString::from(format!("--sub-file={}", subtitle.url)));
                }
                args.push(OsString::from(&stream.url));
            }
            Self::Vlc => {
                if let Some(referer) = stream.referer.as_deref() {
                    args.push(OsString::from(format!("--http-referrer={referer}")));
                }
                if let Some(subtitle) = subtitle {
                    args.push(OsString::from(format!("--sub-file={}", subtitle.url)));
                }
                args.push(OsString::from(&stream.url));
            }
        }
        args
    }

    pub fn play(self, stream: &Stream, subtitle: Option<&Subtitle>, title: &str) -> io::Result<ExitStatus> {
        Command::new(self.command_name())
            .args(self.arguments(stream, subtitle, title))
            .status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream() -> Stream {
        Stream::new("https://cdn.example/video.m3u8", Some("1080p".into()), Some("https://embed.example/".into()))
    }

    #[test]
    fn mpv_arguments_preserve_stream_referer_and_subtitle() {
        let subtitle = Subtitle::new("https://cdn.example/sub.vtt", Some("English".into()));
        let args = Player::Mpv.arguments(&stream(), Some(&subtitle), "Example Anime");
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
        assert!(args.contains(&"--referrer=https://embed.example/".to_owned()));
        assert!(args.contains(&"--sub-file=https://cdn.example/sub.vtt".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some("https://cdn.example/video.m3u8"));
    }

    #[test]
    fn vlc_arguments_include_referer_when_required() {
        let args = Player::Vlc.arguments(&stream(), None, "Example Anime");
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
        assert!(args.contains(&"--http-referrer=https://embed.example/".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some("https://cdn.example/video.m3u8"));
    }

    #[test]
    fn selects_mpv_by_default_and_vlc_when_requested() {
        assert_eq!(Player::from_vlc_flag(false), Player::Mpv);
        assert_eq!(Player::from_vlc_flag(true), Player::Vlc);
    }
}
