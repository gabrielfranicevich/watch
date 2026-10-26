use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::time::Duration;

use crate::domain::stream::Stream;
use reqwest::blocking::Client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfmpegDownloader;

impl FfmpegDownloader {
    pub fn arguments(stream: &Stream, destination: &Path) -> Vec<OsString> {
        Self::arguments_with_extra(stream, destination, &[])
    }

    pub fn arguments_with_extra(
        stream: &Stream,
        destination: &Path,
        extra_flags: &[String],
    ) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("-hide_banner"),
            OsString::from("-y"),
            OsString::from("-extension_picky"),
            OsString::from("0"),
        ];
        if let Some(referer) = stream.referer.as_deref() {
            args.extend([OsString::from("-referer"), OsString::from(referer)]);
        }
        args.extend([
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-stats"),
            OsString::from("-i"),
            OsString::from(&stream.url),
        ]);

        if let Some(audio_url) = stream.audio_url.as_deref() {
            args.extend([OsString::from("-extension_picky"), OsString::from("0")]);
            if let Some(referer) = stream.referer.as_deref() {
                args.extend([OsString::from("-referer"), OsString::from(referer)]);
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
        args.extend(extra_flags.iter().map(OsString::from));
        args
    }

    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        Self::download_with_extra(stream, destination, &[])
    }

    pub fn download_with_extra(
        stream: &Stream,
        destination: &Path,
        extra_flags: &[String],
    ) -> io::Result<ExitStatus> {
        Command::new("ffmpeg")
            .args(Self::arguments_with_extra(stream, destination, extra_flags))
            .status()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YtDlpDownloader;

impl YtDlpDownloader {
    pub fn arguments(stream: &Stream, destination: &Path) -> Vec<OsString> {
        Self::arguments_with_extra(stream, destination, &[])
    }

    pub fn arguments_with_extra(
        stream: &Stream,
        destination: &Path,
        extra_flags: &[String],
    ) -> Vec<OsString> {
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
        args.extend(extra_flags.iter().map(OsString::from));
        args
    }

    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        Self::download_with_extra(stream, destination, &[])
    }

    pub fn download_with_extra(
        stream: &Stream,
        destination: &Path,
        extra_flags: &[String],
    ) -> io::Result<ExitStatus> {
        Command::new("yt-dlp")
            .args(Self::arguments_with_extra(stream, destination, extra_flags))
            .status()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubtitleDownloader;

impl SubtitleDownloader {
    pub fn download(url: &str, referer: Option<&str>, destination: &Path) -> io::Result<()> {
        let client = Client::builder()
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/124.0.0.0 Safari/537.36",
            )
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(io::Error::other)?;

        let mut request = client.get(url);
        if let Some(referer) = referer {
            request = request.header(reqwest::header::REFERER, referer);
        }
        let response = request
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(io::Error::other)?;
        let content = response.bytes().map_err(io::Error::other)?;
        std::fs::write(destination, content)
    }
}

/// Prefer yt-dlp for a single HLS source and fall back to FFmpeg. When a
/// separate audio rendition exists, FFmpeg must mux the two inputs explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDownloader;

impl MediaDownloader {
    pub fn download(stream: &Stream, destination: &Path) -> io::Result<ExitStatus> {
        Self::download_with_extra(stream, destination, &[])
    }

    fn should_try_yt_dlp(stream: &Stream) -> bool {
        stream.audio_url.is_none()
    }

    pub fn download_with_extra(
        stream: &Stream,
        destination: &Path,
        extra_flags: &[String],
    ) -> io::Result<ExitStatus> {
        if Self::should_try_yt_dlp(stream) {
            match YtDlpDownloader::download_with_extra(stream, destination, extra_flags) {
                Ok(status) if status.success() => return Ok(status),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        FfmpegDownloader::download_with_extra(stream, destination, extra_flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_download_forwards_referer_and_writes_response() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{SystemTime, UNIX_EPOCH};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            connection
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let count = connection.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]).into_owned();
            let body = "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n";
            write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Type: text/vtt\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            request
        });

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let destination = std::env::temp_dir().join(format!(
            "ani-cli-subtitle-{}-{nonce}.vtt",
            std::process::id()
        ));
        SubtitleDownloader::download(
            &format!("http://{address}/subtitle.vtt"),
            Some("https://embed.example/"),
            &destination,
        )
        .unwrap();
        let request = server.join().unwrap();

        assert!(request.starts_with("GET /subtitle.vtt HTTP/1.1"));
        assert!(request.contains("referer: https://embed.example/"));
        assert!(request.to_ascii_lowercase().contains("user-agent:"));
        assert_eq!(
            std::fs::read_to_string(&destination).unwrap(),
            "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHello\n"
        );
        std::fs::remove_file(destination).unwrap();
    }

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
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-referer" && pair[1] == "https://embed.example/")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-extension_picky" && pair[1] == "0")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "-loglevel" && pair[1] == "error")
        );
        assert!(args.contains(&"-stats".to_owned()));
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
        assert!(!args.iter().any(|arg| arg == "-referer"));
    }

    #[test]
    fn yt_dlp_passes_referer_and_uses_fragment_retries() {
        let stream = Stream::new(
            "https://cdn.example/episode.m3u8?token=a&b=c",
            None,
            Some("https://embed.example/".into()),
        );
        let args = YtDlpDownloader::arguments_with_extra(
            &stream,
            Path::new("episode.mp4"),
            &["--limit-rate=2M".to_owned()],
        );
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--referer" && pair[1] == "https://embed.example/")
        );
        assert!(args.contains(&"--fragment-retries".to_owned()));
        assert!(args.contains(&"infinite".to_owned()));
        assert!(args.contains(&"-N".to_owned()));
        assert_eq!(args[args.len() - 3], "-o");
        assert_eq!(args.last().map(String::as_str), Some("--limit-rate=2M"));
        assert!(args.contains(&"--limit-rate=2M".to_owned()));
    }

    #[test]
    fn ffmpeg_appends_custom_flags_after_the_output_path() {
        let stream = Stream::new("https://cdn.example/episode.m3u8", None, None);
        let args = FfmpegDownloader::arguments_with_extra(
            &stream,
            Path::new("episode.mp4"),
            &["-preset".to_owned(), "fast".to_owned()],
        );
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(&args[args.len() - 3..], ["episode.mp4", "-preset", "fast"]);
    }

    #[test]
    fn yt_dlp_omits_referer_when_not_provided() {
        let stream = Stream::new("https://cdn.example/episode.m3u8", None, None);
        let args = YtDlpDownloader::arguments(&stream, Path::new("episode.mp4"));
        assert!(!args.iter().any(|arg| arg == "--referer"));
    }
    #[test]
    fn media_downloader_uses_yt_dlp_only_for_single_streams() {
        let video_only = Stream::new(
            "https://cdn.example/video.m3u8",
            Some("1080p".into()),
            Some("https://embed.example/".into()),
        );
        let separate_audio = video_only
            .clone()
            .with_audio(Some("https://cdn.example/audio.m3u8".into()));

        assert!(MediaDownloader::should_try_yt_dlp(&video_only));
        assert!(!MediaDownloader::should_try_yt_dlp(&separate_audio));
    }

}
