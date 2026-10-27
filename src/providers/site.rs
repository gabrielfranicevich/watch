use std::{collections::HashSet, fmt, time::Duration};

use reqwest::{Url, blocking::Client};
use scraper::{Html, Selector};

use crate::domain::{anime::Anime, episode::Episode, stream::Stream, subtitle::Subtitle};
use crate::media::hls::{into_stream, parse_default_audio_url, parse_master_playlist, select_quality};
use super::hianime_stream::{HiAnimeStreamResolver, QualityOption, ResolvedMedia, StreamResolveError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Spanish,
}

impl Language {
    pub fn short_code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    HiAnime,
    GoGoAnime,
    AniWatch,
    NineAnime,
    AnimeKai,
    MonosChinos,
    TioAnime,
    AnimeBlix,
    AnimeFenix,
    JkAnime,
}

impl ProviderId {
    pub const ALL: [Self; 10] = [
        Self::HiAnime, Self::GoGoAnime, Self::AniWatch, Self::NineAnime, Self::AnimeKai,
        Self::MonosChinos, Self::TioAnime, Self::AnimeBlix, Self::AnimeFenix, Self::JkAnime,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::HiAnime => "HiAnime",
            Self::GoGoAnime => "Gogoanime",
            Self::AniWatch => "AniWatch",
            Self::NineAnime => "9anime",
            Self::AnimeKai => "AnimeKai",
            Self::MonosChinos => "MonosChinos",
            Self::TioAnime => "TioAnime",
            Self::AnimeBlix => "Animeblix",
            Self::AnimeFenix => "AnimeFenix",
            Self::JkAnime => "JKAnime",
        }
    }

    pub fn language(self) -> Language {
        match self {
            Self::HiAnime | Self::GoGoAnime | Self::AniWatch | Self::NineAnime | Self::AnimeKai => Language::English,
            Self::MonosChinos | Self::TioAnime | Self::AnimeBlix | Self::AnimeFenix | Self::JkAnime => Language::Spanish,
        }
    }

    pub fn base_url(self) -> &'static str {
        match self {
            Self::HiAnime => "https://hianime.at",
            Self::GoGoAnime => "https://gogoanime.by",
            Self::AniWatch => "https://aniwatchtv.ro",
            Self::NineAnime => "https://9anime.si",
            Self::AnimeKai => "https://animekai.be",
            Self::MonosChinos => "https://monoschinos.st",
            Self::TioAnime => "https://tioanime.com",
            Self::AnimeBlix => "https://animeblix.net",
            Self::AnimeFenix => "https://ww1.animefenix.vip",
            Self::JkAnime => "https://jkanime.net",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::HiAnime => "hianime",
            Self::GoGoAnime => "gogoanime",
            Self::AniWatch => "aniwatch",
            Self::NineAnime => "9anime",
            Self::AnimeKai => "animekai",
            Self::MonosChinos => "monoschinos",
            Self::TioAnime => "tioanime",
            Self::AnimeBlix => "animeblix",
            Self::AnimeFenix => "animefenix",
            Self::JkAnime => "jkanime",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|provider| provider.key() == key)
    }

    fn search_routes(self) -> &'static [SearchRoute] {
        match self {
            Self::HiAnime => &[],
            Self::GoGoAnime => &[
                SearchRoute { path: "/search.html", key: Some("keyword"), slug: false },
                SearchRoute { path: "/", key: Some("s"), slug: false },
            ],
            Self::AniWatch | Self::NineAnime => &[
                SearchRoute { path: "/search", key: Some("keyword"), slug: false },
                SearchRoute { path: "/search", key: Some("q"), slug: false },
            ],
            Self::AnimeKai => &[
                SearchRoute { path: "/search", key: Some("keyword"), slug: false },
                SearchRoute { path: "/", key: Some("s"), slug: false },
            ],
            Self::MonosChinos => &[
                SearchRoute { path: "/", key: Some("s"), slug: false },
                SearchRoute { path: "/buscar", key: Some("q"), slug: false },
            ],
            Self::TioAnime => &[
                SearchRoute { path: "/buscar", key: Some("q"), slug: false },
                SearchRoute { path: "/buscar", key: Some("query"), slug: false },
                SearchRoute { path: "/buscar", key: None, slug: true },
            ],
            Self::AnimeBlix => &[
                SearchRoute { path: "/buscar", key: Some("q"), slug: false },
                SearchRoute { path: "/", key: Some("s"), slug: false },
                SearchRoute { path: "/buscar", key: None, slug: true },
            ],
            Self::AnimeFenix => &[
                SearchRoute { path: "/", key: Some("s"), slug: false },
                SearchRoute { path: "/buscar", key: Some("query"), slug: false },
                SearchRoute { path: "/search", key: Some("keyword"), slug: false },
            ],
            Self::JkAnime => &[
                SearchRoute { path: "/buscar", key: None, slug: true },
                SearchRoute { path: "/buscar", key: Some("q"), slug: false },
                SearchRoute { path: "/", key: Some("s"), slug: false },
            ],
        }
    }

    fn is_detail_path(self, url: &Url) -> bool {
        let path = url.path().to_ascii_lowercase();
        let episode_path = ["/ver/", "/ver-", "/episode-", "-episode-", "/episodio-", "/ep-"]
            .iter().any(|marker| path.contains(marker));
        match self {
            Self::HiAnime => path.starts_with("/watch/") && !episode_path,
            Self::GoGoAnime => path.starts_with("/series/"),
            Self::AniWatch => path.starts_with("/category/") || path.starts_with("/anime/")
                || (path.starts_with("/watch/") && !episode_path),
            Self::NineAnime => path.starts_with("/anime/")
                || (path.starts_with("/watch/") && !episode_path),
            Self::AnimeKai => path.starts_with("/anime/")
                || (path.starts_with("/watch/") && !path.contains("/ep-")),
            Self::MonosChinos => path.starts_with("/anime/"),
            Self::TioAnime => path.starts_with("/anime/") || path.starts_with("/animes/"),
            Self::AnimeBlix => (path.starts_with("/animes/") || path.starts_with("/anime/")) && !path.contains("/ver-"),
            Self::AnimeFenix => path.starts_with("/anime/") || path.starts_with("/animes/"),
            Self::JkAnime => {
                path.trim_matches('/').split('/').filter(|segment| !segment.is_empty()).count() == 1
                    && !["buscar", "anime", "animes", "login", "registro", "calendario", "emision"]
                        .iter().any(|route| path.trim_matches('/') == *route)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SearchRoute {
    path: &'static str,
    key: Option<&'static str>,
    slug: bool,
}

#[derive(Debug, Clone)]
pub struct SiteError(String);

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SiteError {}

#[derive(Debug, Clone)]
pub struct SiteProvider {
    id: ProviderId,
    client: Client,
    base_url: String,
}

impl SiteProvider {
    pub fn new(id: ProviderId) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36")
            .timeout(Duration::from_secs(8))
            .build().expect("static HTTP client configuration is valid");
        Self { id, client, base_url: id.base_url().to_owned() }
    }

    pub fn search(&self, query: &str) -> Result<Vec<Anime>, SiteError> {
        let mut last_error = "no search route returned recognizable results".to_owned();
        for route in self.id.search_routes() {
            let url = self.search_url(route, query)?;
            let response = match self.client.get(url).send() {
                Ok(response) => response,
                Err(error) => {
                    last_error = format!("request failed: {error}");
                    continue;
                }
            };
            let status = response.status();
            let body = match response.text() {
                Ok(body) => body,
                Err(error) => {
                    last_error = format!("could not read response: {error}");
                    continue;
                }
            };
            if is_verification_page(&body) {
                last_error = "site returned an anti-bot verification page".to_owned();
                continue;
            }
            if !status.is_success() {
                last_error = format!("site returned HTTP status {status}");
                continue;
            }
            let results = parse_search_results(&body, &self.base_url, self.id, query);
            if !results.is_empty() {
                return Ok(results);
            }
        }
        Err(SiteError(format!("{} search failed: {last_error}", self.id.name())))
    }

    fn search_url(&self, route: &SearchRoute, query: &str) -> Result<Url, SiteError> {
        let mut url = Url::parse(&self.base_url)
            .and_then(|base| base.join(route.path))
            .map_err(|error| SiteError(format!("invalid provider URL: {error}")))?;
        if route.slug {
            let slug = query.split_whitespace().collect::<Vec<_>>().join("-");
            url.set_path(&format!("{}/{}", route.path.trim_end_matches('/'), slug));
        } else if let Some(key) = route.key {
            url.query_pairs_mut().append_pair(key, query);
        }
        Ok(url)
    }

    pub fn episodes(&self, anime: &Anime) -> Result<Vec<Episode>, SiteError> {
        let response = self.client.get(&anime.url).send()
            .map_err(|error| SiteError(format!("{} episode request failed: {error}", self.id.name())))?;
        let status = response.status();
        let body = response.text()
            .map_err(|error| SiteError(format!("{} episode response failed: {error}", self.id.name())))?;
        if is_verification_page(&body) {
            return Err(SiteError(format!("{} returned an anti-bot verification page", self.id.name())));
        }
        if !status.is_success() {
            return Err(SiteError(format!("{} returned HTTP status {status} while loading episodes", self.id.name())));
        }

        let mut episodes = parse_episode_links(&body, &self.base_url, &anime.url);
        if episodes.is_empty() {
            if let Ok(url) = Url::parse(&anime.url) {
                if is_episode_page(&url) {
                    if let Some(number) = episode_number_from("", url.as_str()) {
                        let document = Html::parse_document(&body);
                        episodes.push(Episode::new(number, first_text(&document, &["h1", ".entry-title"]), anime.url.clone()));
                    }
                }
            }
        }
        if episodes.is_empty() {
            return Err(SiteError(format!(
                "{} returned no recognizable episode links for '{}'; its page structure may have changed",
                self.id.name(), anime.title
            )));
        }
        Ok(episodes)
    }
}

fn is_verification_page(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    ["just a moment", "cf-browser-verification", "challenge-platform", "verify you are human", "checking your browser"]
        .iter().any(|marker| lower.contains(marker))
}

fn parse_search_results(body: &str, base_url: &str, provider: ProviderId, query: &str) -> Vec<Anime> {
    let document = Html::parse_document(body);
    let Ok(anchors) = Selector::parse("a[href]") else { return Vec::new() };
    let Ok(base) = Url::parse(base_url) else { return Vec::new() };
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for anchor in document.select(&anchors) {
        let Some(href) = anchor.value().attr("href") else { continue };
        let Ok(url) = base.join(href) else { continue };
        if url.host_str() != base.host_str() || !provider.is_detail_path(&url) { continue; }

        let text = normalize_whitespace(&anchor.text().collect::<String>());
        let title = anchor.value().attr("title").map(str::trim)
            .filter(|value| !value.is_empty()).map(str::to_owned).unwrap_or(text);
        if !plausible_title(&title) || !matches_query(&title, query) { continue; }
        let id = url.path().trim_matches('/').to_owned();
        if !id.is_empty() && seen.insert(id.clone()) {
            results.push(Anime::new(id, title, url.to_string()));
        }
    }
    results
}

fn title_key(value: &str) -> String {
    value.to_lowercase().chars().filter(|character| character.is_alphanumeric()).collect()
}

fn matches_query(title: &str, query: &str) -> bool {
    let key = title_key(title);
    let query_key = title_key(query);
    (!query_key.is_empty() && key.contains(&query_key))
        || query.split_whitespace().all(|term| {
            let term = title_key(term);
            !term.is_empty() && key.contains(&term)
        })
}

fn plausible_title(title: &str) -> bool {
    let lower = title.trim().to_ascii_lowercase();
    title.chars().any(char::is_alphanumeric) && title.trim().len() < 180
        && !["home", "search", "search anime", "buscar", "inicio", "catalog", "catalogo", "anime list", "login", "register", "watch now", "ver ahora", "más info"]
            .iter().any(|blocked| lower == *blocked)
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_episode_page(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    ["/ver/", "/ver-", "/episode-", "-episode-", "/episodio-", "/ep-"]
        .iter().any(|marker| path.contains(marker))
}

fn parse_episode_links(body: &str, base_url: &str, anime_url: &str) -> Vec<Episode> {
    let document = Html::parse_document(body);
    let Ok(anchors) = Selector::parse("a[href]") else { return Vec::new() };
    let Ok(base) = Url::parse(base_url) else { return Vec::new() };
    let mut episodes = Vec::new();
    let mut seen = HashSet::new();

    for anchor in document.select(&anchors) {
        let Some(href) = anchor.value().attr("href") else { continue };
        let Ok(url) = base.join(href) else { continue };
        if url.host_str() != base.host_str() || url.as_str() == anime_url { continue; }
        let label = normalize_whitespace(&anchor.text().collect::<String>());
        let path = url.path().to_ascii_lowercase();
        let episode_like_path = is_episode_page(&url)
            || path.contains("/episode/")
            || path.contains("/episodes/")
            || path.contains("/watch/")
            || path.contains("/ver/")
            || path.contains("/ep/")
            || path.contains("/capitulo/")
            || path.contains("/episodio/");
        let Some(number) = episode_number_from(&label, url.as_str()) else { continue };
        let label_is_number = !label.trim().is_empty() && label.trim().chars().all(|character| character.is_ascii_digit());
        if !(episode_like_path || has_episode_marker(&label) || label_is_number) { continue; }
        if label_is_number && !episode_like_path { continue; }
        let key = format!("{number}:{}", url.as_str());
        if !seen.insert(key) { continue; }

        let title = if has_episode_marker(&label) {
            let remainder = episode_title_remainder(&label);
            (!remainder.is_empty()).then(|| remainder.to_owned())
        } else {
            None
        };
        episodes.push(Episode::new(number, title, url.to_string()));
    }

    episodes.sort_by(|left, right| left.number.parse::<f64>().unwrap_or(f64::MAX)
        .total_cmp(&right.number.parse::<f64>().unwrap_or(f64::MAX))
        .then_with(|| left.url.cmp(&right.url)));
    episodes
}

fn has_episode_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["episode", "episodio", "episódio", "capitulo", "capítulo", "chapter", "ep."]
        .iter().any(|marker| lower.contains(marker))
}

fn episode_title_remainder(value: &str) -> &str {
    let lower = value.to_ascii_lowercase();
    for marker in ["episode", "episodio", "episódio", "capitulo", "capítulo", "chapter", "ep."] {
        if let Some(index) = lower.find(marker) {
            return value[index + marker.len()..]
                .trim_start_matches(|character: char| character.is_ascii_digit() || character.is_whitespace() || matches!(character, ':' | '-' | '#' | '.')).trim();
        }
    }
    ""
}

fn episode_number_from(label: &str, url: &str) -> Option<String> {
    let lower = label.to_ascii_lowercase();
    for marker in ["episode", "episodio", "episódio", "chapter", "capitulo", "capítulo", "ep-"] {
        if let Some(index) = lower.find(marker) {
            if let Some(number) = first_number(&lower[index + marker.len()..]) { return Some(number); }
        }
    }
    if !label.trim().is_empty() && label.trim().chars().all(|character| character.is_ascii_digit()) {
        return Some(label.trim().to_owned());
    }
    let parsed = Url::parse(url).ok()?;
    let path = parsed.path().trim_end_matches('/');
    if !is_episode_page(&parsed)
        && !path.contains("/episode/")
        && !path.contains("/episodes/")
        && !path.contains("/watch/")
        && !path.contains("/ver/")
        && !path.contains("/ep/")
        && !path.contains("/capitulo/")
        && !path.contains("/episodio/")
    {
        return None;
    }
    path.rsplit('/').next()?.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty()).rev()
        .find(|part| part.chars().all(|character| character.is_ascii_digit())).map(str::to_owned)
}

fn first_number(value: &str) -> Option<String> {
    let mut number = String::new();
    let mut started = false;
    for character in value.chars() {
        if character.is_ascii_digit() || (started && character == '.') { number.push(character); started = true; }
        else if started { break; }
    }
    (!number.is_empty() && number != ".").then_some(number)
}

fn first_text(document: &Html, selectors: &[&str]) -> Option<String> {
    selectors.iter().find_map(|selector| {
        let selector = Selector::parse(selector).ok()?;
        let value = document.select(&selector).next()?;
        let text = normalize_whitespace(&value.text().collect::<String>());
        (!text.is_empty()).then_some(text)
    })
}

#[derive(Debug, Clone)]
pub enum ProviderStreamResolver {
    HiAnime(HiAnimeStreamResolver),
    Site(SiteStreamResolver),
}

impl ProviderStreamResolver {
    pub fn new(provider: ProviderId) -> Self {
        if provider == ProviderId::HiAnime { Self::HiAnime(HiAnimeStreamResolver::new()) }
        else { Self::Site(SiteStreamResolver::new(provider)) }
    }

    pub fn resolve(&self, episode: &Episode, dubbed: bool, quality: &str) -> Result<ResolvedMedia, StreamResolveError> {
        match self {
            Self::HiAnime(resolver) => resolver.resolve(episode, dubbed, quality),
            Self::Site(resolver) => resolver.resolve(episode, dubbed, quality),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SiteStreamResolver {
    provider: ProviderId,
    client: Client,
}

impl SiteStreamResolver {
    pub fn new(provider: ProviderId) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36")
            .timeout(Duration::from_secs(12))
            .build().expect("static HTTP client configuration is valid");
        Self { provider, client }
    }

    pub fn resolve(&self, episode: &Episode, _dubbed: bool, quality: &str) -> Result<ResolvedMedia, StreamResolveError> {
        self.find_media(&episode.url, None, quality, 0, &mut HashSet::new())
    }

    fn find_media(&self, page_url: &str, referer: Option<&str>, quality: &str, depth: usize, seen: &mut HashSet<String>)
        -> Result<ResolvedMedia, StreamResolveError>
    {
        if depth > 4 { return Err(StreamResolveError::InvalidData("embed nesting exceeded the supported depth".into())); }
        if !seen.insert(page_url.to_owned()) { return Err(StreamResolveError::InvalidData("repeated embed URL".into())); }

        let response = self.client.get(page_url)
            .header(reqwest::header::REFERER, referer.unwrap_or(self.provider.base_url())).send()?;
        let status = response.status();
        let body = response.text()?;
        if is_verification_page(&body) {
            return Err(StreamResolveError::InvalidData(format!("{} returned an anti-bot verification page", self.provider.name())));
        }
        if !status.is_success() {
            return Err(StreamResolveError::InvalidData(format!("{} returned HTTP status {status} while resolving the stream", self.provider.name())));
        }

        let document = Html::parse_document(&body);
        let subtitle = find_subtitle(&document, page_url);
        let media_candidates = extract_media_urls(&body, page_url);
        for candidate in media_candidates.iter().filter(|url| url.to_ascii_lowercase().contains(".m3u8")) {
            if let Ok(mut media) = self.resolve_media_url(candidate, page_url, quality) {
                if media.subtitle.is_none() { media.subtitle = subtitle.clone(); }
                return Ok(media);
            }
        }
        for candidate in media_candidates.iter().filter(|url| url.to_ascii_lowercase().contains(".mp4")) {
            if let Ok(mut media) = self.resolve_media_url(candidate, page_url, quality) {
                if media.subtitle.is_none() { media.subtitle = subtitle.clone(); }
                return Ok(media);
            }
        }

        let selector = Selector::parse("iframe[src], video[src], video source[src], source[src]")
            .map_err(|_| StreamResolveError::InvalidData("invalid media selector".into()))?;
        let page = Url::parse(page_url).map_err(|error| StreamResolveError::InvalidData(format!("invalid episode URL: {error}")))?;
        let mut last_error = None;
        for element in document.select(&selector) {
            let Some(src) = element.value().attr("src") else { continue };
            let Ok(next) = page.join(src) else { continue };
            if !matches!(next.scheme(), "http" | "https") || !is_public_web_url(&next) { continue; }
            match self.find_media(next.as_str(), Some(page_url), quality, depth + 1, seen) {
                Ok(mut media) => {
                    if media.subtitle.is_none() { media.subtitle = subtitle.clone(); }
                    return Ok(media);
                }
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        Err(StreamResolveError::InvalidData(last_error.unwrap_or_else(|| format!(
            "{} did not expose a directly extractable HLS or MP4 source; its player may need a dedicated resolver", self.provider.name()
        ))))
    }

    fn resolve_media_url(&self, media_url: &str, referer: &str, quality: &str) -> Result<ResolvedMedia, StreamResolveError> {
        if media_url.to_ascii_lowercase().contains(".mp4") {
            return Ok(ResolvedMedia { stream: Stream::new(media_url, None, Some(referer.into())), subtitle: None, mal_id: None, qualities: Vec::new() });
        }

        let response = self.client.get(media_url).header(reqwest::header::REFERER, referer).send()?;
        let status = response.status();
        let playlist = response.text()?;
        if is_verification_page(&playlist) || !status.is_success() {
            return Err(StreamResolveError::InvalidData(format!("media URL returned HTTP status {status}")));
        }
        let variants = match parse_master_playlist(&playlist, media_url) {
            Ok(variants) => variants,
            Err(_) if playlist.lines().any(|line| line.trim() == "#EXTM3U") => {
                return Ok(ResolvedMedia { stream: Stream::new(media_url, None, Some(referer.into())), subtitle: None, mal_id: None, qualities: Vec::new() });
            }
            Err(error) => return Err(StreamResolveError::Hls(error)),
        };
        let selected = select_quality(&variants, quality).or_else(|| select_quality(&variants, "best"))
            .ok_or_else(|| StreamResolveError::InvalidData("HLS playlist has no selectable quality".into()))?;
        let mut stream = into_stream(selected, Some(referer.to_owned()));
        stream.audio_url = parse_default_audio_url(&playlist, media_url).ok().flatten();
        let mut qualities: Vec<QualityOption> = variants.iter().map(|variant| {
            let value = variant.resolution.as_deref()
                .and_then(|resolution| resolution.split_once('x').map(|(_, height)| height.to_owned()))
                .map(|height| format!("{height}p")).unwrap_or_else(|| "best".into());
            QualityOption { label: variant.resolution.clone().unwrap_or_else(|| value.clone()), value }
        }).collect();
        qualities.sort_by(|left, right| right.value.trim_end_matches('p').parse::<u32>().unwrap_or_default()
            .cmp(&left.value.trim_end_matches('p').parse::<u32>().unwrap_or_default()));
        qualities.dedup_by(|left, right| left.value == right.value);
        Ok(ResolvedMedia { stream, subtitle: None, mal_id: None, qualities })
    }
}

fn is_public_web_url(url: &Url) -> bool {
    if let Some(host) = url.host_str() {
        let host = host.to_ascii_lowercase();
        if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") { return false; }
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return match ip {
                std::net::IpAddr::V4(ip) => !ip.is_private() && !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified() && !ip.is_broadcast(),
                std::net::IpAddr::V6(ip) => !ip.is_loopback() && !ip.is_unspecified() && !ip.is_unique_local() && !ip.is_unicast_link_local(),
            };
        }
    }
    true
}

fn find_subtitle(document: &Html, page_url: &str) -> Option<Subtitle> {
    let selector = Selector::parse("track[src]").ok()?;
    let page = Url::parse(page_url).ok()?;
    document.select(&selector).find_map(|element| {
        let value = element.value();
        let src = value.attr("src")?;
        let url = page.join(src).ok()?;
        Some(Subtitle::new(url.to_string(), value.attr("label").or_else(|| value.attr("srclang")).map(str::to_owned)))
    })
}

fn extract_media_urls(body: &str, page_url: &str) -> Vec<String> {
    let normalized = body.replace("\\/", "/").replace("&amp;", "&").replace("&#038;", "&");
    let Ok(page) = Url::parse(page_url) else { return Vec::new() };
    let mut urls = Vec::new();
    for candidate in normalized.split(|character: char| character.is_whitespace() || matches!(character, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}')) {
        let candidate = candidate.trim_matches(|character: char| matches!(character, ',' | ';' | ':' | '!'));
        let lower = candidate.to_ascii_lowercase();
        if !lower.contains(".m3u8") && !lower.contains(".mp4") { continue; }
        let candidate = candidate.trim_end_matches(['.', ',', ';']);
        let Ok(url) = page.join(candidate) else { continue };
        if matches!(url.scheme(), "http" | "https") && is_public_web_url(&url) && !urls.contains(&url.to_string()) {
            urls.push(url.to_string());
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_metadata_matches_requested_language_groups() {
        assert_eq!(ProviderId::GoGoAnime.language(), Language::English);
        assert_eq!(ProviderId::AniWatch.language(), Language::English);
        assert_eq!(ProviderId::NineAnime.language(), Language::English);
        assert_eq!(ProviderId::AnimeKai.language(), Language::English);
        assert_eq!(ProviderId::MonosChinos.language(), Language::Spanish);
        assert_eq!(ProviderId::TioAnime.language(), Language::Spanish);
        assert_eq!(ProviderId::AnimeBlix.language(), Language::Spanish);
        assert_eq!(ProviderId::AnimeFenix.language(), Language::Spanish);
        assert_eq!(ProviderId::JkAnime.language(), Language::Spanish);
        assert_eq!(ProviderId::ALL.len(), 10);
    }

    #[test]
    fn search_parser_ignores_navigation_and_deduplicates_provider_cards() {
        let html = r#"<nav><a href="/series/">Search</a></nav>
            <div><a href="/series/frieren/" title="Frieren: Beyond Journey's End">Frieren</a>
            <a href="/series/frieren/" title="Duplicate">Frieren</a></div>"#;
        let results = parse_search_results(html, "https://gogoanime.by", ProviderId::GoGoAnime, "Frieren");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Frieren: Beyond Journey's End");
    }

    #[test]
    fn episode_parser_sorts_numeric_episodes_and_skips_navigation() {
        let html = r#"<a href="/watch/frieren/ep-10">Episode 10</a>
            <a href="/watch/frieren/ep-2">Episode 2</a>
            <a href="/watch/frieren/ep-1">Episode 1</a><a href="/about">About</a>"#;
        let episodes = parse_episode_links(html, "https://animekai.be", "https://animekai.be/watch/frieren");
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].number, "1");
        assert_eq!(episodes[1].number, "2");
        assert_eq!(episodes[2].number, "10");
    }

    #[test]
    fn extracts_escaped_media_urls_and_preserves_query_strings() {
        let found = extract_media_urls(
            r#"<script>var src = "https:\/\/cdn.example\/video\/master.m3u8?token=abc&amp;x=1";</script>"#,
            "https://example.test/watch/episode");
        assert_eq!(found, vec!["https://cdn.example/video/master.m3u8?token=abc&x=1"]);
    }

    #[test]
    fn does_not_follow_private_or_local_embed_urls() {
        assert!(!is_public_web_url(&Url::parse("http://127.0.0.1/player").unwrap()));
        assert!(!is_public_web_url(&Url::parse("http://192.168.1.1/player").unwrap()));
        assert!(is_public_web_url(&Url::parse("https://player.example/watch").unwrap()));
    }
}
