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
#[path = "../../tests/unit/media/downloader.rs"]
mod tests;
