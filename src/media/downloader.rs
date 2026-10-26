use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::domain::stream::Stream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfmpegDownloader;

impl FfmpegDownloader {
    pub fn arguments(stream: &Stream, destination: &Path) -> Vec<OsString> {
        let mut args = vec![OsString::from("-hide_banner"), OsString::from("-y")];
        if let Some(referer) = stream.referer.as_deref() {
            args.push(OsString::from("-headers"));
            args.push(OsString::from(format!("Referer: {referer}\r\n")));
        }
        args.extend([OsString::from("-i"), OsString::from(&stream.url)]);

        if let Some(audio_url) = stream.audio_url.as_deref() {
            if let Some(referer) = stream.referer.as_deref() {
                args.push(OsString::from("-headers"));
                args.push(OsString::from(format!("Referer: {referer}\r\n")));
            }
            args.extend([OsString::from("-i"), OsString::from(audio_url)]);
            args.extend([
                OsString::from("-map"),
                OsString::from("0:v:0"),
                OsString::from("-map"),
                OsString::from("1:a:0"),
            ]);
        }

        args.extend([
            OsString::from("-c"),
            OsString::from("copy"),
            destination.as_os_str().to_owned(),
        ]);
        args
    }

    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        Command::new("ffmpeg")
            .args(Self::arguments(stream, destination))
            .status()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YtDlpDownloader;

impl YtDlpDownloader {
    pub fn arguments(stream: &Stream, destination: &Path) -> Vec<OsString> {
        let mut args = Vec::new();
        if let Some(referer) = stream.referer.as_deref() {
            args.extend([OsString::from("--referer"), OsString::from(referer)]);
        }
        args.extend([
            OsString::from(&stream.url),
            OsString::from("--no-skip-unavailable-fragments"),
            OsString::from("--fragment-retries"),
            OsString::from("infinite"),
            OsString::from("-N"),
            OsString::from("16"),
            OsString::from("-o"),
            destination.as_os_str().to_owned(),
        ]);
        args
    }

    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        Command::new("yt-dlp")
            .args(Self::arguments(stream, destination))
            .status()
    }
}

/// Prefer yt-dlp for a single HLS source and fall back to FFmpeg. When a
/// separate audio rendition exists, FFmpeg must mux the two inputs explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDownloader;

impl MediaDownloader {
    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        if stream.audio_url.is_none() {
            match YtDlpDownloader::download(stream, destination) {
                Ok(status) if status.success() => return Ok(status),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        FfmpegDownloader::download(stream, destination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_forwards_referer_and_stream_as_distinct_arguments() {
        let stream = Stream::new(
            "https://cdn.example/episode.m3u8?token=a&b=c",
            None,
            Some("https://embed.example/".into()),
        );
        let args = FfmpegDownloader::arguments(&stream, Path::new("episode.mkv"));
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|pair| {
            pair[0] == "-headers" && pair[1] == "Referer: https://embed.example/\r\n"
        }));
        assert!(args.windows(2).any(
            |pair| pair[0] == "-i" && pair[1] == "https://cdn.example/episode.m3u8?token=a&b=c"
        ));
        assert_eq!(args.last().map(String::as_str), Some("episode.mkv"));
    }

    #[test]
    fn maps_video_and_external_audio_when_present() {
        let stream = Stream::new(
            "https://cdn.example/video.m3u8",
            Some("1080p".into()),
            Some("https://embed.example/".into()),
        )
        .with_audio(Some("https://cdn.example/audio.m3u8".into()));
        let args = FfmpegDownloader::arguments(&stream, Path::new("episode.mp4"));
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-i" && pair[1] == "https://cdn.example/audio.m3u8")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-map" && pair[1] == "0:v:0")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-map" && pair[1] == "1:a:0")
        );
    }

    #[test]
    fn ffmpeg_supports_streams_without_referer() {
        let stream = Stream::new("https://cdn.example/episode.m3u8", None, None);
        let args = FfmpegDownloader::arguments(&stream, Path::new("episode.mp4"));
        assert!(!args.iter().any(|arg| arg == "-headers"));
    }

    #[test]
    fn yt_dlp_passes_referer_and_uses_fragment_retries() {
        let stream = Stream::new(
            "https://cdn.example/episode.m3u8?token=a&b=c",
            None,
            Some("https://embed.example/".into()),
        );
        let args = YtDlpDownloader::arguments(&stream, Path::new("episode.mp4"));
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(
            |pair| pair[0] == "--referer" && pair[1] == "https://embed.example/"
        ));
        assert!(args.contains(&"--fragment-retries".to_owned()));
        assert!(args.contains(&"infinite".to_owned()));
        assert!(args.contains(&"-N".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some("episode.mp4"));
    }

    #[test]
    fn yt_dlp_omits_referer_when_not_provided() {
        let stream = Stream::new("https://cdn.example/episode.m3u8", None, None);
        let args = YtDlpDownloader::arguments(&stream, Path::new("episode.mp4"));
        assert!(!args.iter().any(|arg| arg == "--referer"));
    }
}
