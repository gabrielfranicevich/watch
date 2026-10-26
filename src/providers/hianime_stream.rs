use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::time::Duration;

use crate::domain::{episode::Episode, stream::Stream, subtitle::Subtitle};
use super::hianime::is_browser_verification_page;
use crate::media::hls::{
    HlsVariant, into_stream, parse_default_audio_url, parse_master_playlist, select_quality,
};
use super::hianime::is_browser_verification_page;

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
pub struct QualityOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMedia {
    pub stream: Stream,
    pub subtitle: Option<Subtitle>,
    pub mal_id: Option<String>,
    pub qualities: Vec<QualityOption>,
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
        let servers_response = self.client.get(servers_url).send()?;
        let servers_body = provider_response_body(servers_response)?;
        let servers = Html::parse_fragment(&normalize_server_html(&servers_body));
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
        let mal_id = mal_id_from_embed(&embed_url);

        let embed_response = self.client.get(&embed_url).send()?;
        let embed_body = provider_response_body(embed_response)?;
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

        let playlist_response = self
            .client
            .get(master_url)
            .header(reqwest::header::REFERER, &referer)
            .send()?;
        let playlist = provider_response_body(playlist_response)?;
        let variants = parse_master_playlist(&playlist, master_url)?;
        let qualities = quality_options(&variants);
        let audio_url = parse_default_audio_url(&playlist, master_url)?;
        let selected = select_quality(&variants, quality)
            .or_else(|| select_quality(&variants, "best"))
            .ok_or_else(|| {
                StreamResolveError::InvalidData(
                    "HLS playlist contains no selectable quality".into(),
                )
            })?;
        let stream = into_stream(selected, Some(referer)).with_audio(audio_url);

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

        Ok(ResolvedMedia {
            stream,
            subtitle,
            mal_id,
            qualities,
        })
    }
}

fn quality_options(variants: &[HlsVariant]) -> Vec<QualityOption> {
    let mut options: Vec<(u32, QualityOption)> = variants
        .iter()
        .filter_map(|variant| {
            let height = variant
                .resolution
                .as_deref()?
                .split_once('x')?
                .1
                .parse::<u32>()
                .ok()?;
            let value = format!("{height}p");
            Some((
                height,
                QualityOption {
                    label: value.clone(),
                    value,
                },
            ))
        })
        .collect();

    options.sort_by_key(|option| std::cmp::Reverse(option.0));
    options.dedup_by(|left, right| left.1.value == right.1.value);
    options.into_iter().map(|(_, option)| option).collect()
}

fn provider_response_body(
    response: reqwest::blocking::Response,
) -> Result<String, StreamResolveError> {
    let status = response.status();
    let body = response.text()?;

    if is_browser_verification_page(&body) {
        return Err(StreamResolveError::InvalidData(
            "HiAnime returned an anti-bot verification page while resolving the stream; retry later or use a compatible curl-impersonate setup".into(),
        ));
    }
    if !status.is_success() {
        return Err(StreamResolveError::InvalidData(format!(
            "HiAnime stream endpoint returned HTTP status {status}"
        )));
    }

    Ok(body)
}

fn normalize_server_html(body: &str) -> String {
    body.replace('\\', "")
}

fn extract_window_blob(html: &str) -> Option<&str> {
    let marker = "window.__P=\"";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn mal_id_from_embed(url: &str) -> Option<String> {
    let id = url.split("/mal/").nth(1)?.split('/').next()?;
    (!id.is_empty() && id.chars().all(|character| character.is_ascii_digit()))
        .then(|| id.to_owned())
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
    fn extracts_unique_quality_choices_in_descending_resolution_order() {
        let playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=100000,RESOLUTION=640x360\n360.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=1280x720\n720-a.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=950000,RESOLUTION=1280x720\n720-b.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1920x1080\n1080.m3u8\n";
        let variants = parse_master_playlist(playlist, "https://cdn.example/master.m3u8").unwrap();
        let qualities = quality_options(&variants);
        assert_eq!(
            qualities
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["1080p", "720p", "360p"]
        );
        assert!(qualities.iter().all(|option| option.label == option.value));
    }

    #[test]
    fn omits_quality_variants_without_a_known_resolution() {
        let playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=100000\nunknown.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=1280x720\n720.m3u8\n";
        let variants = parse_master_playlist(playlist, "https://cdn.example/master.m3u8").unwrap();
        assert_eq!(
            quality_options(&variants)
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["720p"]
        );
    }

    fn mock_http_response(
        status: u16,
        body: &'static str,
    ) -> (String, std::thread::JoinHandle<()>) {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            write!(
                connection,
                "HTTP/1.1 {status} Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        (format!("http://{address}/error"), handle)
    }

    fn mock_stream_resolver_server() -> (String, std::thread::JoinHandle<Vec<String>>) {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let embed_url = format!("{base_url}/mal/12345/watch");
        let server_hash = BASE64.encode(embed_url.as_bytes());

        let config = serde_json::json!({
            "src": format!("{base_url}/hls/master.m3u8"),
            "subtitles": [{
                "src": format!("{base_url}/subtitles/en.vtt"),
                "default": true
            }]
        })
        .to_string();
        let encrypted: Vec<u8> = config
            .bytes()
            .enumerate()
            .map(|(index, byte)| byte ^ XOR_KEY[index % XOR_KEY.len()])
            .collect();
        let embed_blob = BASE64.encode(encrypted);

        let servers_body = format!(
            r#"<div class="server-item" data-type="sub" data-server-name="ZokoAnime" data-hash="{server_hash}"></div>"#
        );
        let embed_body = format!(r#"<script>window.__P="{embed_blob}";</script>"#);
        let playlist = String::from(
            "#EXTM3U\n\
             #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"English\",DEFAULT=YES,URI=\"../audio/en.m3u8\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=500000,RESOLUTION=640x360\n\
             ../variants/360/index.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=1280x720\n\
             ../variants/720/index.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1920x1080\n\
             ../variants/1080/index.m3u8\n",
        );

        let worker = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..3 {
                let (mut connection, _) = listener.accept().unwrap();
                connection
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0_u8; 8192];
                let count = connection.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]).into_owned();
                let request_path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_owned();
                let (status, content_type, body) =
                    if request_path.starts_with("/api/theme/episode/servers?episodeId=1001") {
                        ("200 OK", "text/html; charset=utf-8", servers_body.as_str())
                    } else if request_path == "/mal/12345/watch" {
                        ("200 OK", "text/html; charset=utf-8", embed_body.as_str())
                    } else if request_path == "/hls/master.m3u8" {
                        ("200 OK", "application/vnd.apple.mpegurl", playlist.as_str())
                    } else {
                        ("404 Not Found", "text/plain; charset=utf-8", "not found")
                    };
                write!(
                    connection,
                    "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                connection.flush().unwrap();
                requests.push(format!("{request_path}\n{request}"));
            }
            requests
        });

        (base_url, worker)
    }

    #[test]
    fn resolves_server_embed_hls_audio_subtitles_and_available_qualities() {
        let (base_url, server) = mock_stream_resolver_server();
        let resolver = HiAnimeStreamResolver::with_base_url(&base_url);
        let episode = Episode::new("1", None, format!("{base_url}/watch/frieren-123?ep=1001"));

        let resolved = resolver.resolve(&episode, false, "720p").unwrap();
        let requests = server.join().unwrap();

        assert_eq!(
            resolved.stream.url,
            format!("{base_url}/variants/720/index.m3u8")
        );
        assert_eq!(
            resolved.stream.audio_url.as_deref(),
            Some(format!("{base_url}/audio/en.m3u8").as_str())
        );
        assert_eq!(
            resolved
                .subtitle
                .as_ref()
                .map(|subtitle| subtitle.url.as_str()),
            Some(format!("{base_url}/subtitles/en.vtt").as_str())
        );
        assert_eq!(resolved.mal_id.as_deref(), Some("12345"));
        assert_eq!(
            resolved
                .qualities
                .iter()
                .map(|option| option.value.as_str())
                .collect::<Vec<_>>(),
            vec!["1080p", "720p", "360p"]
        );
        assert!(requests[0].starts_with("/api/theme/episode/servers?episodeId=1001\n"));
        assert!(requests[1].starts_with("/mal/12345/watch\n"));
        assert!(requests[2].starts_with("/hls/master.m3u8\n"));
    }

    #[test]
    fn removes_json_escape_backslashes_before_parsing_server_html() {
        let body = r#"<div class=\"server-item\" data-type=\"sub\" data-server-name=\"ZokoAnime\" data-hash=\"YWJj\"></div>"#;
        let normalized = normalize_server_html(body);
        let document = Html::parse_fragment(&normalized);
        let selector = Selector::parse(".server-item").unwrap();
        let server = document.select(&selector).next().unwrap();
        assert_eq!(server.value().attr("data-type"), Some("sub"));
        assert_eq!(server.value().attr("data-hash"), Some("YWJj"));
    }

    #[test]
    fn stream_response_reports_anti_bot_challenges_and_http_errors() {
        use std::io::Write;
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut connection, _) = listener.accept().unwrap();
            connection
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let body = "<title>Just a moment...</title>";
            write!(
                connection,
                "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let response = reqwest::blocking::Client::new()
            .get(format!("http://{address}/challenge"))
            .send()
            .unwrap();
        let error = provider_response_body(response).unwrap_err();
        server.join().unwrap();
        assert!(error.to_string().contains("anti-bot verification"));

        let (stream, _server) = mock_http_response(403, "forbidden");
        let response = reqwest::blocking::Client::new()
            .get(&stream)
            .send()
            .unwrap();
        let error = provider_response_body(response).unwrap_err();
        assert!(error.to_string().contains("HTTP status 403"));
    }

    #[test]
    fn extracts_embed_blob_without_consuming_following_markup() {
        assert_eq!(
            extract_window_blob("<script>window.__P=\"YWJjZA==\";</script>"),
            Some("YWJjZA==")
        );
        assert_eq!(extract_window_blob("<html></html>"), None);
    }

    #[test]
    fn extracts_mal_id_from_embed_url_when_available() {
        assert_eq!(
            mal_id_from_embed("https://embed.example/mal/12345/watch"),
            Some("12345".to_owned())
        );
        assert_eq!(mal_id_from_embed("https://embed.example/watch"), None);
        assert_eq!(
            mal_id_from_embed("https://embed.example/mal/nope/watch"),
            None
        );
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
