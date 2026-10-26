use std::error::Error;
use std::fmt;

use crate::domain::stream::Stream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsVariant {
    pub bandwidth: u64,
    pub resolution: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlsError(String);

impl fmt::Display for HlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for HlsError {}

pub fn parse_master_playlist(body: &str, master_url: &str) -> Result<Vec<HlsVariant>, HlsError> {
    if !body.lines().any(|line| line.trim() == "#EXTM3U") {
        return Err(HlsError("response is not an HLS playlist".into()));
    }

    let mut variants = Vec::new();
    let mut pending: Option<(u64, Option<String>)> = None;

    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(attributes) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let bandwidth = attribute(attributes, "BANDWIDTH")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let resolution = attribute(attributes, "RESOLUTION").map(str::to_owned);
            pending = Some((bandwidth, resolution));
        } else if !line.starts_with('#') {
            if let Some((bandwidth, resolution)) = pending.take() {
                variants.push(HlsVariant {
                    bandwidth,
                    resolution,
                    url: resolve_url(master_url, line)?,
                });
            }
        }
    }

    if variants.is_empty() {
        return Err(HlsError("HLS master playlist contains no stream variants".into()));
    }
    Ok(variants)
}

pub fn select_quality<'a>(variants: &'a [HlsVariant], requested: &str) -> Option<&'a HlsVariant> {
    if variants.is_empty() {
        return None;
    }
    match requested {
        "best" => variants.iter().max_by_key(|variant| variant.bandwidth),
        "worst" => variants.iter().min_by_key(|variant| variant.bandwidth),
        value => variants.iter().find(|variant| {
            variant.resolution.as_deref().is_some_and(|resolution| resolution.starts_with(value))
                || resolution_height(variant.resolution.as_deref()) == Some(value.trim_end_matches('p'))
        }),
    }
}

pub fn into_stream(variant: &HlsVariant, referer: Option<String>) -> Stream {
    Stream::new(variant.url.clone(), variant.resolution.clone(), referer)
}

fn resolution_height(resolution: Option<&str>) -> Option<&str> {
    resolution?.split_once('x').map(|(_, height)| height)
}

fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    attributes.split(',').find_map(|item| {
        let (key, value) = item.trim().split_once('=')?;
        (key == name).then_some(value.trim_matches('"'))
    })
}

fn resolve_url(base: &str, reference: &str) -> Result<String, HlsError> {
    if reference.starts_with("https://") || reference.starts_with("http://") {
        return Ok(reference.to_owned());
    }
    let (origin, path) = base
        .split_once("://")
        .ok_or_else(|| HlsError("invalid master playlist URL".into()))?;
    let host_end = path.find('/').unwrap_or(path.len());
    let authority = &path[..host_end];
    let base_path = &path[host_end..];
    if reference.starts_with("//") {
        return Ok(format!("{origin}:{reference}"));
    }
    if reference.starts_with('/') {
        return Ok(format!("{origin}://{authority}{reference}"));
    }
    let directory = base_path.rsplit_once('/').map(|(directory, _)| directory).unwrap_or("");
    let joined = if directory.is_empty() {
        format!("/{reference}")
    } else {
        format!("{directory}/{reference}")
    };
    Ok(format!("{origin}://{authority}{}", normalize_path(&joined)))
}

fn normalize_path(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => { parts.pop(); }
            _ => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYLIST: &str = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360/index.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1920x1080\n../1080/index.m3u8\n";

    #[test]
    fn parses_variants_and_resolves_relative_urls() {
        let variants = parse_master_playlist(PLAYLIST, "https://cdn.example/hls/master.m3u8").unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].url, "https://cdn.example/hls/360/index.m3u8");
        assert_eq!(variants[1].url, "https://cdn.example/1080/index.m3u8");
    }

    #[test]
    fn selects_best_worst_and_requested_height() {
        let variants = parse_master_playlist(PLAYLIST, "https://cdn.example/hls/master.m3u8", None).unwrap();
        assert_eq!(select_quality(&variants, "best").unwrap().resolution.as_deref(), Some("1920x1080"));
        assert_eq!(select_quality(&variants, "worst").unwrap().resolution.as_deref(), Some("640x360"));
        assert_eq!(select_quality(&variants, "720p"), None);
        assert_eq!(select_quality(&variants, "1080p").unwrap().bandwidth, 2500000);
    }

    #[test]
    fn rejects_non_playlist_response() {
        assert!(parse_master_playlist("not a playlist", "https://cdn.example/master").is_err());
    }
}
