use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;

use crate::domain::{episode::Episode, stream::Stream, subtitle::Subtitle};
use crate::media::hls::{into_stream, parse_master_playlist, select_quality};

const XOR_KEY: &[u8] = b"otaku-embed-v1";

#[derive(Debug)]
pub enum StreamResolveError {
    Request(reqwest::Error),
    InvalidData(String),
    Hls(crate::media::hls::HlsError),
}

impl std::fmt::Display for StreamResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(error) => write!(f, "HiAnime request failed: {error}"),
            Self::InvalidData(message) => f.write_str(message),
            Self::Hls(error) => write!(f, "invalid HLS playlist: {error}"),
        }
    }
}

impl std::error::Error for StreamResolveError {}

impl From<reqwest::Error> for StreamResolveError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error)
    }
}

impl From<crate::media::hls::HlsError> for StreamResolveError {
    fn from(error: crate::media::hls::HlsError) -> Self {
        Self::Hls(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMedia {
    pub stream: Stream,
    pub subtitle: Option<Subtitle>,
}

#[derive(Debug, Clone)]
pub struct HiAnimeStreamResolver {
    client: Client,
    base_url: String,
}

impl Default for HiAnimeStreamResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl HiAnimeStreamResolver {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("static HTTP client configuration is valid");
        Self {
            client,
            base_url: "https://hianime.at".to_owned(),
        }
    }

    #[cfg(test)]
    fn with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    pub fn resolve(
        &self,
        episode: &Episode,
        dubbed: bool,
        quality: &str,
    ) -> Result<ResolvedMedia, StreamResolveError> {
        let episode_id = episode
            .url
            .split("?ep=")
            .nth(1)
            .and_then(|value| value.split('&').next())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                StreamResolveError::InvalidData("episode URL is missing the ?ep= identifier".into())
            })?;
        let servers_url = format!(
            "{}/api/theme/episode/servers?episodeId={episode_id}",
            self.base_url
        );
        let servers_body = self
            .client
            .get(servers_url)
            .send()?
            .error_for_status()?
            .text()?;
        let servers = Html::parse_fragment(&servers_body.replace("\\\\", ""));
        let selector = Selector::parse(".server-item")
            .map_err(|_| StreamResolveError::InvalidData("invalid server selector".into()))?;
        let mode = if dubbed { "dub" } else { "sub" };
        let encoded_hash = servers
            .select(&selector)
            .find_map(|element| {
                let value = element.value();
                (value.attr("data-type") == Some(mode)
                    && value.attr("data-server-name") == Some("ZokoAnime"))
                .then(|| value.attr("data-hash"))
                .flatten()
            })
            .ok_or_else(|| {
                StreamResolveError::InvalidData(format!(
                    "HiAnime has no ZokoAnime {mode} server for this episode"
                ))
            })?;

        let embed_bytes = STANDARD.decode(encoded_hash).map_err(|_| {
            StreamResolveError::InvalidData("HiAnime returned an invalid server hash".into())
        })?;
        let embed_url = String::from_utf8(embed_bytes).map_err(|_| {
            StreamResolveError::InvalidData("HiAnime server URL is not UTF-8".into())
        })?;
        let referer = origin_with_slash(&embed_url)?;

        let embed_body = self
            .client
            .get(&embed_url)
            .send()?
            .error_for_status()?
            .text()?;
        let blob = extract_window_blob(&embed_body).ok_or_else(|| {
            StreamResolveError::InvalidData("HiAnime embed page did not contain window.__P".into())
        })?;
        let decoded = STANDARD.decode(blob).map_err(|_| {
            StreamResolveError::InvalidData("HiAnime embed config is not valid base64".into())
        })?;
        let json_bytes: Vec<u8> = decoded
            .iter()
            .enumerate()
            .map(|(index, byte)| *byte ^ XOR_KEY[index % XOR_KEY.len()])
            .collect();
        let config: Value = serde_json::from_slice(&json_bytes).map_err(|error| {
            StreamResolveError::InvalidData(format!(
                "HiAnime embed config is invalid JSON: {error}"
            ))
        })?;
        let master_url = config
            .get("src")
            .and_then(Value::as_str)
            .filter(|url| url.contains(".m3u8"))
            .ok_or_else(|| {
                StreamResolveError::InvalidData("HiAnime embed config has no HLS source".into())
            })?;

        let playlist = self
            .client
            .get(master_url)
            .header(reqwest::header::REFERER, &referer)
            .send()?
            .error_for_status()?
            .text()?;
        let variants = parse_master_playlist(&playlist, master_url)?;
        let selected = select_quality(&variants, quality)
            .or_else(|| select_quality(&variants, "best"))
            .ok_or_else(|| {
                StreamResolveError::InvalidData(
                    "HLS playlist contains no selectable quality".into(),
                )
            })?;
        let stream = into_stream(selected, Some(referer));

        let subtitle = config
            .get("subtitles")
            .and_then(Value::as_array)
            .and_then(|subtitles| {
                subtitles
                    .iter()
                    .find(|item| item.get("default").and_then(Value::as_bool) == Some(true))
            })
            .and_then(|item| item.get("src").and_then(Value::as_str))
            .map(|url| Subtitle::new(url, Some("default".into())));

        Ok(ResolvedMedia { stream, subtitle })
    }
}

fn extract_window_blob(html: &str) -> Option<&str> {
    let marker = "window.__P=\"";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn origin_with_slash(url: &str) -> Result<String, StreamResolveError> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| StreamResolveError::InvalidData("invalid embed URL".into()))?;
    let authority = rest
        .split('/')
        .next()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| StreamResolveError::InvalidData("invalid embed URL host".into()))?;
    Ok(format!("{scheme}://{authority}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_embed_blob_without_consuming_following_markup() {
        assert_eq!(
            extract_window_blob("<script>window.__P=\"YWJjZA==\";</script>"),
            Some("YWJjZA==")
        );
        assert_eq!(extract_window_blob("<html></html>"), None);
    }

    #[test]
    fn derives_embed_origin_for_referer() {
        assert_eq!(
            origin_with_slash("https://embed.example/path/watch?id=1").unwrap(),
            "https://embed.example/"
        );
        assert!(origin_with_slash("relative/path").is_err());
    }

    #[test]
    fn rejects_episode_urls_without_provider_episode_id() {
        let resolver = HiAnimeStreamResolver::with_base_url("http://127.0.0.1:1234");
        let episode = Episode::new("1", None, "https://hianime.at/watch/example");
        assert!(matches!(
            resolver.resolve(&episode, false, "best"),
            Err(StreamResolveError::InvalidData(_))
        ));
    }
}
