use std::{collections::HashSet, fmt, thread, time::Duration};

use reqwest::{Url, blocking::Client};
use scraper::{Html, Selector};

use super::hianime_stream::{
  HiAnimeStreamResolver, QualityOption, ResolvedMedia, StreamResolveError,
};
use crate::domain::{anime::Anime, episode::Episode, stream::Stream, subtitle::Subtitle};
use crate::media::hls::{
  into_stream, parse_default_audio_url, parse_master_playlist, select_quality,
};

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
    Self::HiAnime,
    Self::GoGoAnime,
    Self::AniWatch,
    Self::NineAnime,
    Self::AnimeKai,
    Self::MonosChinos,
    Self::TioAnime,
    Self::AnimeBlix,
    Self::AnimeFenix,
    Self::JkAnime,
  ];

  /// Providers currently enabled for search. Legacy providers remain in `ALL`
  /// so history entries can still be read without querying unavailable sites.
  pub const ACTIVE: [Self; 5] = [
    Self::HiAnime,
    Self::GoGoAnime,
    Self::AniWatch,
    Self::TioAnime,
    Self::JkAnime,
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
      Self::HiAnime | Self::GoGoAnime | Self::AniWatch | Self::NineAnime | Self::AnimeKai => {
        Language::English
      }
      Self::MonosChinos | Self::TioAnime | Self::AnimeBlix | Self::AnimeFenix | Self::JkAnime => {
        Language::Spanish
      }
    }
  }

  pub fn base_url(self) -> &'static str {
    match self {
      Self::HiAnime => "https://hianime.at",
      Self::GoGoAnime => "https://gogoanime.by",
      Self::AniWatch => "https://aniwatchtv.ro",
      Self::NineAnime => "https://9anime.si",
      Self::AnimeKai => "https://anikai.to",
      Self::MonosChinos => "https://monoschinos.st",
      Self::TioAnime => "https://tioanime.com",
      Self::AnimeBlix => "https://animeblix.net",
      Self::AnimeFenix => "https://ww1.animefenix.vip",
      Self::JkAnime => "https://jkanime.surf",
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
        SearchRoute {
          path: "/search.html",
          key: Some("keyword"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
      ],
      Self::AniWatch | Self::NineAnime => &[
        SearchRoute {
          path: "/search",
          key: Some("keyword"),
          slug: false,
        },
        SearchRoute {
          path: "/search",
          key: Some("q"),
          slug: false,
        },
      ],
      Self::AnimeKai => &[
        SearchRoute {
          path: "/search",
          key: Some("keyword"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
      ],
      Self::MonosChinos => &[
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("q"),
          slug: false,
        },
      ],
      Self::TioAnime => &[
        SearchRoute {
          path: "/directorio",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("query"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: None,
          slug: true,
        },
      ],
      Self::AnimeBlix => &[
        SearchRoute {
          path: "/",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("search"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: None,
          slug: true,
        },
      ],
      Self::AnimeFenix => &[
        SearchRoute {
          path: "/",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("search"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("query"),
          slug: false,
        },
        SearchRoute {
          path: "/search",
          key: Some("keyword"),
          slug: false,
        },
      ],
      Self::JkAnime => &[
        SearchRoute {
          path: "/buscar",
          key: None,
          slug: true,
        },
        SearchRoute {
          path: "/buscar",
          key: Some("q"),
          slug: false,
        },
        SearchRoute {
          path: "/",
          key: Some("s"),
          slug: false,
        },
      ],
    }
  }

  fn is_detail_path(self, url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    let episode_path = [
      "/ver/",
      "/ver-",
      "/episode-",
      "-episode-",
      "/episodio-",
      "-episodio-",
      "/ep-",
    ]
    .iter()
    .any(|marker| path.contains(marker));
    match self {
      Self::HiAnime => path.starts_with("/watch/") && !episode_path,
      Self::GoGoAnime => path.starts_with("/series/"),
      Self::AniWatch => {
        path.starts_with("/category/")
          || path.starts_with("/anime/")
          || (path.starts_with("/watch/") && !episode_path)
      }
      Self::NineAnime => {
        path.starts_with("/anime/") || (path.starts_with("/watch/") && !episode_path)
      }
      Self::AnimeKai => {
        path.starts_with("/anime/") || (path.starts_with("/watch/") && !path.contains("/ep-"))
      }
      Self::MonosChinos => path.starts_with("/anime/"),
      Self::TioAnime => path.starts_with("/anime/") || path.starts_with("/animes/"),
      Self::AnimeBlix => {
        (path.starts_with("/animes/") || path.starts_with("/anime/")) && !path.contains("/ver-")
      }
      Self::AnimeFenix => path.starts_with("/anime/") || path.starts_with("/animes/"),
      Self::JkAnime => {
        path.starts_with("/anime/")
          || (path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            == 1
            && ![
              "buscar",
              "anime",
              "animes",
              "login",
              "registro",
              "calendario",
              "emision",
            ]
            .iter()
            .any(|route| path.trim_matches('/') == *route))
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
      .user_agent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36",
      )
      .timeout(Duration::from_secs(8))
      .build()
      .expect("static HTTP client configuration is valid");
    Self {
      id,
      client,
      base_url: id.base_url().to_owned(),
    }
  }

  pub fn search(&self, query: &str) -> Result<Vec<Anime>, SiteError> {
    let routes: Vec<_> = self
      .id
      .search_routes()
      .iter()
      .map(|route| self.search_url(route, query).map(|url| (*route, url)))
      .collect::<Result<_, _>>()?;

    // Search every supported route concurrently. Sites often change their search
    // parameter or path, and a later route can return results the first one misses.
    let handles: Vec<_> = routes
      .into_iter()
      .map(|(_route, url)| {
        let client = self.client.clone();
        let provider = self.id;
        let query = query.to_owned();
        thread::spawn(move || {
          let response = client.get(url).send().map_err(|error| {
            SiteError(format!(
              "{} search request failed: {error}",
              provider.name()
            ))
          })?;
          let status = response.status();
          let final_url = response.url().to_string();
          let body = response.text().map_err(|error| {
            SiteError(format!(
              "{} search response failed: {error}",
              provider.name()
            ))
          })?;
          if is_verification_page(&body) {
            return Err(SiteError(format!(
              "{} returned an anti-bot verification page",
              provider.name()
            )));
          }
          if !status.is_success() {
            return Err(SiteError(format!(
              "{} returned HTTP status {status}",
              provider.name()
            )));
          }
          let results = parse_search_results(&body, &final_url, provider, &query);
          Ok(results)
        })
      })
      .collect();

    let mut results = Vec::new();
    let mut last_error = "no search route returned recognizable results".to_owned();
    for handle in handles {
      match handle.join() {
        Ok(Ok(route_results)) => results.extend(route_results),
        Ok(Err(error)) => last_error = error.to_string(),
        Err(_) => last_error = "search route worker panicked".to_owned(),
      }
    }

    let mut seen = HashSet::new();
    results.retain(|anime| seen.insert(anime.id.clone()));
    if results.is_empty() {
      return Err(SiteError(format!(
        "{} search failed: {last_error}",
        self.id.name()
      )));
    }
    Ok(results)
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
    let response = self.client.get(&anime.url).send().map_err(|error| {
      SiteError(format!(
        "{} episode request failed: {error}",
        self.id.name()
      ))
    })?;
    let status = response.status();
    let body = response.text().map_err(|error| {
      SiteError(format!(
        "{} episode response failed: {error}",
        self.id.name()
      ))
    })?;
    if is_verification_page(&body) {
      return Err(SiteError(format!(
        "{} returned an anti-bot verification page",
        self.id.name()
      )));
    }
    if !status.is_success() {
      return Err(SiteError(format!(
        "{} returned HTTP status {status} while loading episodes",
        self.id.name()
      )));
    }

    let mut episodes = parse_episode_links(&body, &anime.url, &anime.url);
    if self.id == ProviderId::TioAnime && episodes.is_empty() {
      episodes = parse_tioanime_episodes(&body, &anime.url);
    }
    if episodes.is_empty() {
      if let Ok(url) = Url::parse(&anime.url) {
        if is_episode_page(&url) {
          if let Some(number) = episode_number_from("", url.as_str()) {
            let document = Html::parse_document(&body);
            episodes.push(Episode::new(
              number,
              first_text(&document, &["h1", ".entry-title"]),
              anime.url.clone(),
            ));
          }
        }
      }
    }
    if episodes.is_empty() {
      if self.id == ProviderId::TioAnime {
        if let Some(date) = next_episode_date(&body) {
          return Err(SiteError(format!(
            "TioAnime has no released episodes for '{}'; the next episode is scheduled for {date}",
            anime.title
          )));
        }
      }
      return Err(SiteError(format!(
        "{} returned no recognizable episode links for '{}'; its page structure may have changed",
        self.id.name(),
        anime.title
      )));
    }
    Ok(episodes)
  }
}

fn is_verification_page(body: &str) -> bool {
  let lower = body.to_ascii_lowercase();
  [
    "just a moment",
    "cf-browser-verification",
    "challenge-platform",
    "verify you are human",
    "checking your browser",
  ]
  .iter()
  .any(|marker| lower.contains(marker))
}

fn parse_search_results(
  body: &str,
  base_url: &str,
  provider: ProviderId,
  query: &str,
) -> Vec<Anime> {
  let document = Html::parse_document(body);
  let Ok(anchors) = Selector::parse("a[href]") else {
    return Vec::new();
  };
  let Ok(base) = Url::parse(base_url) else {
    return Vec::new();
  };
  let mut results = Vec::new();
  let mut seen = HashSet::new();

  for anchor in document.select(&anchors) {
    let Some(href) = anchor.value().attr("href") else {
      continue;
    };
    let Ok(url) = base.join(href) else { continue };
    if url.host_str() != base.host_str() || !provider.is_detail_path(&url) {
      continue;
    }

    let text = normalize_whitespace(&anchor.text().collect::<String>());
    let title = anchor
      .value()
      .attr("title")
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map(str::to_owned)
      .unwrap_or(text);
    if !plausible_title(&title) || !matches_query(&title, query) {
      continue;
    }
    let id = url.path().trim_matches('/').to_owned();
    if !id.is_empty() && seen.insert(id.clone()) {
      results.push(Anime::new(id, title, url.to_string()));
    }
  }
  results
}

fn title_key(value: &str) -> String {
  value
    .to_lowercase()
    .chars()
    .filter(|character| character.is_alphanumeric())
    .collect()
}

fn matches_query(title: &str, query: &str) -> bool {
  let key = title_key(title);
  let query_key = title_key(query);
  if query_key.is_empty() {
    return false;
  }

  key.contains(&query_key)
    || query.split_whitespace().all(|term| {
      let term = title_key(term);
      !term.is_empty() && key.contains(&term)
    })
}

fn plausible_title(title: &str) -> bool {
  let lower = title.trim().to_ascii_lowercase();
  title.chars().any(char::is_alphanumeric)
    && title.trim().len() < 180
    && ![
      "home",
      "search",
      "search anime",
      "buscar",
      "inicio",
      "catalog",
      "catalogo",
      "anime list",
      "login",
      "register",
      "watch now",
      "ver ahora",
      "más info",
    ]
    .iter()
    .any(|blocked| lower == *blocked)
}

fn normalize_whitespace(value: &str) -> String {
  value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_episode_page(url: &Url) -> bool {
  let path = url.path().to_ascii_lowercase();
  [
    "/ver/",
    "/ver-",
    "/episode-",
    "-episode-",
    "/episodio-",
    "-episodio-",
    "/ep-",
  ]
  .iter()
  .any(|marker| path.contains(marker))
}

fn parse_episode_links(body: &str, base_url: &str, anime_url: &str) -> Vec<Episode> {
  let document = Html::parse_document(body);
  let Ok(anchors) = Selector::parse("a[href]") else {
    return Vec::new();
  };
  let Ok(base) = Url::parse(base_url) else {
    return Vec::new();
  };
  let mut episodes = Vec::new();
  let mut seen = HashSet::new();
  let anime_slug = Url::parse(anime_url)
    .ok()
    .and_then(|url| {
      url
        .path()
        .trim_matches('/')
        .rsplit('/')
        .next()
        .map(str::to_ascii_lowercase)
    })
    .filter(|slug| !slug.is_empty());
  let episode_series_slug = anime_slug.as_deref().map(|slug| {
    if base
      .host_str()
      .is_some_and(|host| host.contains("monoschinos"))
    {
      slug.strip_suffix("-sub-espanol").unwrap_or(slug)
    } else {
      slug
    }
  });

  for anchor in document.select(&anchors) {
    let Some(href) = anchor.value().attr("href") else {
      continue;
    };
    let Ok(url) = base.join(href) else { continue };
    if url.host_str() != base.host_str() || url.as_str() == anime_url {
      continue;
    }
    let path = url.path().to_ascii_lowercase();
    let episode_like_path = is_episode_page(&url)
      || path.contains("/episode/")
      || path.contains("/episodes/")
      || path.contains("/watch/")
      || path.contains("/ver/")
      || path.contains("/ep/")
      || path.contains("/capitulo/")
      || path.contains("/episodio/");
    if !episode_like_path {
      continue;
    }
    // Provider detail pages often include recommendations and latest-episode
    // widgets. Require the candidate URL to retain the selected anime's slug
    // so those widgets cannot inject episode numbers from unrelated series.
    if episode_series_slug.is_some_and(|slug| !path.contains(slug)) {
      continue;
    }

    let label = normalize_whitespace(&anchor.text().collect::<String>());
    let Some(number) = episode_number_from(&label, url.as_str()) else {
      continue;
    };
    let key = number.clone();
    if !seen.insert(key) {
      continue;
    }

    let title = if has_episode_marker(&label) {
      let remainder = episode_title_remainder(&label);
      (!remainder.is_empty()).then(|| remainder.to_owned())
    } else {
      None
    };
    episodes.push(Episode::new(number, title, url.to_string()));
  }

  episodes.sort_by(|left, right| {
    left
      .number
      .parse::<f64>()
      .unwrap_or(f64::MAX)
      .total_cmp(&right.number.parse::<f64>().unwrap_or(f64::MAX))
      .then_with(|| left.url.cmp(&right.url))
  });
  episodes
}

fn parse_tioanime_episodes(body: &str, anime_url: &str) -> Vec<Episode> {
  let Ok(anime) = Url::parse(anime_url) else {
    return Vec::new();
  };
  let Some(slug) = anime.path().trim_matches('/').rsplit('/').next() else {
    return Vec::new();
  };
  if slug.is_empty() {
    return Vec::new();
  }

  let Some(values) = js_numeric_array(body, "episodes") else {
    return Vec::new();
  };
  let mut numbers: Vec<_> = values
    .into_iter()
    .filter(|number| number.parse::<f64>().is_ok())
    .collect();
  numbers.sort_by(|left, right| {
    left
      .parse::<f64>()
      .unwrap_or(f64::MAX)
      .total_cmp(&right.parse::<f64>().unwrap_or(f64::MAX))
  });
  numbers.dedup();

  numbers
    .into_iter()
    .filter_map(|number| {
      let url = anime.join(&format!("/ver/{slug}-{number}")).ok()?;
      Some(Episode::new(number, None, url.to_string()))
    })
    .collect()
}

fn js_numeric_array(body: &str, variable: &str) -> Option<Vec<String>> {
  for (index, _) in body.match_indices(variable) {
    let assignment = body.get(index + variable.len()..)?.trim_start();
    let Some(array) = assignment.strip_prefix('=') else {
      continue;
    };
    let array = array.trim_start().strip_prefix('[')?;
    let values = array.split_once(']')?.0;
    return Some(
      values
        .split(',')
        .map(|value| value.trim().trim_matches(['\'', '"']).to_owned())
        .filter(|value| !value.is_empty())
        .collect(),
    );
  }
  None
}

fn next_episode_date(body: &str) -> Option<String> {
  let lower = body.to_ascii_lowercase();
  let marker_end = ["proximo episodio", "próximo episodio"]
    .iter()
    .find_map(|marker| lower.find(marker).map(|index| index + marker.len()))?;
  body
    .get(marker_end..)?
    .split(|character: char| !character.is_ascii_digit() && character != '-')
    .find_map(|date| {
      let bytes = date.as_bytes();
      (bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
          .iter()
          .enumerate()
          .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit()))
      .then(|| date.to_owned())
    })
}

fn has_episode_marker(value: &str) -> bool {
  let lower = value.to_ascii_lowercase();
  [
    "episode",
    "episodio",
    "episódio",
    "capitulo",
    "capítulo",
    "chapter",
    "ep.",
  ]
  .iter()
  .any(|marker| lower.contains(marker))
}

fn episode_title_remainder(value: &str) -> &str {
  let lower = value.to_ascii_lowercase();
  for marker in [
    "episode",
    "episodio",
    "episódio",
    "capitulo",
    "capítulo",
    "chapter",
    "ep.",
  ] {
    if let Some(index) = lower.find(marker) {
      return value[index + marker.len()..]
        .trim_start_matches(|character: char| {
          character.is_ascii_digit()
            || character.is_whitespace()
            || matches!(character, ':' | '-' | '#' | '.')
        })
        .trim();
    }
  }
  ""
}

fn episode_number_from(label: &str, url: &str) -> Option<String> {
  let lower = label.to_ascii_lowercase();
  for marker in [
    "episode",
    "episodio",
    "episódio",
    "chapter",
    "capitulo",
    "capítulo",
    "ep-",
  ] {
    if let Some(index) = lower.find(marker) {
      if let Some(number) = first_number(&lower[index + marker.len()..]) {
        return Some(number);
      }
    }
  }
  if !label.trim().is_empty()
    && label
      .trim()
      .chars()
      .all(|character| character.is_ascii_digit())
  {
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
  path
    .rsplit('/')
    .next()?
    .split(|character: char| !character.is_ascii_alphanumeric())
    .filter(|part| !part.is_empty())
    .rev()
    .find(|part| part.chars().all(|character| character.is_ascii_digit()))
    .map(str::to_owned)
}

fn first_number(value: &str) -> Option<String> {
  let mut number = String::new();
  let mut started = false;
  for character in value.chars() {
    if character.is_ascii_digit() || (started && character == '.') {
      number.push(character);
      started = true;
    } else if started {
      break;
    }
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
    if provider == ProviderId::HiAnime {
      Self::HiAnime(HiAnimeStreamResolver::new())
    } else {
      Self::Site(SiteStreamResolver::new(provider))
    }
  }

  pub fn resolve(
    &self,
    episode: &Episode,
    dubbed: bool,
    quality: &str,
  ) -> Result<ResolvedMedia, StreamResolveError> {
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
      .user_agent(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36",
      )
      .timeout(Duration::from_secs(12))
      .build()
      .expect("static HTTP client configuration is valid");
    Self { provider, client }
  }

  pub fn resolve(
    &self,
    episode: &Episode,
    _dubbed: bool,
    quality: &str,
  ) -> Result<ResolvedMedia, StreamResolveError> {
    self.find_media(&episode.url, None, quality, 0, &mut HashSet::new())
  }

  fn find_media(
    &self,
    page_url: &str,
    referer: Option<&str>,
    quality: &str,
    depth: usize,
    seen: &mut HashSet<String>,
  ) -> Result<ResolvedMedia, StreamResolveError> {
    if depth > 4 {
      return Err(StreamResolveError::InvalidData(
        "embed nesting exceeded the supported depth".into(),
      ));
    }
    if !seen.insert(page_url.to_owned()) {
      return Err(StreamResolveError::InvalidData("repeated embed URL".into()));
    }

    let response = self
      .client
      .get(page_url)
      .header(
        reqwest::header::REFERER,
        referer.unwrap_or(self.provider.base_url()),
      )
      .send()?;
    let status = response.status();
    let body = response.text()?;
    if is_verification_page(&body) {
      let host = Url::parse(page_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "embed host".into());
      return Err(StreamResolveError::InvalidData(format!(
        "{} embed host {host} returned an anti-bot verification page",
        self.provider.name(),
      )));
    }
    if !status.is_success() {
      return Err(StreamResolveError::InvalidData(format!(
        "{} returned HTTP status {status} while resolving the stream",
        self.provider.name()
      )));
    }

    let document = Html::parse_document(&body);
    let subtitle = find_subtitle(&document, page_url);
    let media_candidates = extract_media_urls(&body, page_url);
    let mut last_error = None;
    for candidate in media_candidates
      .iter()
      .filter(|url| url.to_ascii_lowercase().contains(".m3u8"))
    {
      match self.resolve_media_url(candidate, page_url, quality) {
        Ok(mut media) => {
          if media.subtitle.is_none() {
            media.subtitle = subtitle.clone();
          }
          return Ok(media);
        }
        Err(error) => last_error = Some(error.to_string()),
      }
    }
    for candidate in media_candidates
      .iter()
      .filter(|url| url.to_ascii_lowercase().contains(".mp4"))
    {
      match self.resolve_media_url(candidate, page_url, quality) {
        Ok(mut media) => {
          if media.subtitle.is_none() {
            media.subtitle = subtitle.clone();
          }
          return Ok(media);
        }
        Err(error) => last_error = Some(error.to_string()),
      }
    }

    let page = Url::parse(page_url)
      .map_err(|error| StreamResolveError::InvalidData(format!("invalid episode URL: {error}")))?;
    for src in provider_player_links(&document, &body, self.provider) {
      let Ok(next) = page.join(&src) else { continue };
      if !matches!(next.scheme(), "http" | "https") || !is_public_web_url(&next) {
        continue;
      }
      match self.find_media(next.as_str(), Some(page_url), quality, depth + 1, seen) {
        Ok(mut media) => {
          if media.subtitle.is_none() {
            media.subtitle = subtitle.clone();
          }
          return Ok(media);
        }
        Err(error) => last_error = Some(error.to_string()),
      }
    }
    Err(StreamResolveError::InvalidData(last_error.unwrap_or_else(|| format!(
            "{} did not expose a directly extractable HLS or MP4 source; its player may need a dedicated resolver", self.provider.name()
        ))))
  }

  fn resolve_media_url(
    &self,
    media_url: &str,
    referer: &str,
    quality: &str,
  ) -> Result<ResolvedMedia, StreamResolveError> {
    if is_direct_mp4_url(media_url) {
      let direct_url = Url::parse(media_url)
        .map_err(|error| StreamResolveError::InvalidData(format!("invalid MP4 URL: {error}")))?;
      let response = self
        .client
        .get(media_url)
        .header(reqwest::header::REFERER, referer)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()?;
      if !response.status().is_success() {
        return Err(StreamResolveError::InvalidData(format!(
          "MP4 host {} returned HTTP status {}",
          direct_url.host_str().unwrap_or("unknown"),
          response.status()
        )));
      }
      let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
      if content_type.contains("text/html") || content_type.contains("application/xhtml+xml") {
        return Err(StreamResolveError::InvalidData(format!(
          "MP4 host {} returned HTML instead of a video",
          direct_url.host_str().unwrap_or("unknown")
        )));
      }
      return Ok(ResolvedMedia {
        stream: Stream::new(media_url, None, Some(referer.into())),
        subtitle: None,
        mal_id: None,
        qualities: Vec::new(),
      });
    }

    let response = self
      .client
      .get(media_url)
      .header(reqwest::header::REFERER, referer)
      .send()?;
    let status = response.status();
    let content_type = response
      .headers()
      .get(reqwest::header::CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .unwrap_or_default()
      .to_ascii_lowercase();
    let playlist = response.text()?;
    if content_type.contains("text/html") || content_type.contains("application/xhtml+xml") {
      let host = Url::parse(media_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "media host".into());
      return Err(StreamResolveError::InvalidData(format!(
        "media host {host} returned HTML instead of a media playlist"
      )));
    }
    if is_verification_page(&playlist) || !status.is_success() {
      return Err(StreamResolveError::InvalidData(format!(
        "media URL returned HTTP status {status}"
      )));
    }
    let variants = match parse_master_playlist(&playlist, media_url) {
      Ok(variants) => variants,
      Err(_) if playlist.lines().any(|line| line.trim() == "#EXTM3U") => {
        return Ok(ResolvedMedia {
          stream: Stream::new(media_url, None, Some(referer.into())),
          subtitle: None,
          mal_id: None,
          qualities: Vec::new(),
        });
      }
      Err(error) => return Err(StreamResolveError::Hls(error)),
    };
    let selected = select_quality(&variants, quality)
      .or_else(|| select_quality(&variants, "best"))
      .ok_or_else(|| {
        StreamResolveError::InvalidData("HLS playlist has no selectable quality".into())
      })?;
    let selected_url = Url::parse(&selected.url).map_err(|error| {
      StreamResolveError::InvalidData(format!("invalid HLS variant URL: {error}"))
    })?;
    let selected_path = selected_url.path().to_ascii_lowercase();
    if !selected_path.ends_with(".m3u8") && !selected_path.ends_with(".mp4") {
      let response = self
        .client
        .get(selected_url.as_str())
        .header(reqwest::header::REFERER, media_url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()?;
      if !response.status().is_success() {
        return Err(StreamResolveError::InvalidData(format!(
          "HLS variant host {} returned HTTP status {}",
          selected_url.host_str().unwrap_or("unknown"),
          response.status()
        )));
      }
      let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
      if content_type.contains("text/html") || content_type.contains("application/xhtml+xml") {
        return Err(StreamResolveError::InvalidData(format!(
          "HLS variant host {} returned HTML instead of media",
          selected_url.host_str().unwrap_or("unknown")
        )));
      }
    }
    let mut stream = into_stream(selected, Some(referer.to_owned()));
    stream.audio_url = parse_default_audio_url(&playlist, media_url).ok().flatten();
    let mut qualities: Vec<QualityOption> = variants
      .iter()
      .map(|variant| {
        let value = variant
          .resolution
          .as_deref()
          .and_then(|resolution| {
            resolution
              .split_once('x')
              .map(|(_, height)| height.to_owned())
          })
          .map(|height| format!("{height}p"))
          .unwrap_or_else(|| "best".into());
        QualityOption {
          label: variant.resolution.clone().unwrap_or_else(|| value.clone()),
          value,
        }
      })
      .collect();
    qualities.sort_by_key(|quality| {
      std::cmp::Reverse(
        quality
          .value
          .trim_end_matches('p')
          .parse::<u32>()
          .unwrap_or_default(),
      )
    });
    qualities.dedup_by(|left, right| left.value == right.value);
    Ok(ResolvedMedia {
      stream,
      subtitle: None,
      mal_id: None,
      qualities,
    })
  }
}

fn is_public_web_url(url: &Url) -> bool {
  if let Some(host) = url.host_str() {
    let host = host.to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
      return false;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
      return match ip {
        std::net::IpAddr::V4(ip) => {
          !ip.is_private()
            && !ip.is_loopback()
            && !ip.is_link_local()
            && !ip.is_unspecified()
            && !ip.is_broadcast()
        }
        std::net::IpAddr::V6(ip) => {
          !ip.is_loopback()
            && !ip.is_unspecified()
            && !ip.is_unique_local()
            && !ip.is_unicast_link_local()
        }
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
    Some(Subtitle::new(
      url.to_string(),
      value
        .attr("label")
        .or_else(|| value.attr("srclang"))
        .map(str::to_owned),
    ))
  })
}

fn embedded_player_links(document: &Html, provider: ProviderId) -> Vec<String> {
  let selector = if provider == ProviderId::MonosChinos {
    "iframe[src], video[src], video source[src], source[src], a.direct-link[href]"
  } else {
    "iframe[src], video[src], video source[src], source[src]"
  };
  let Ok(selector) = Selector::parse(selector) else {
    return Vec::new();
  };
  document
    .select(&selector)
    .filter_map(|element| {
      let value = element.value();
      value
        .attr("src")
        .or_else(|| {
          (provider == ProviderId::MonosChinos)
            .then(|| value.attr("href"))
            .flatten()
        })
        .map(str::to_owned)
    })
    .collect()
}

fn provider_player_links(document: &Html, body: &str, provider: ProviderId) -> Vec<String> {
  let mut links = embedded_player_links(document, provider);
  if provider == ProviderId::TioAnime {
    links.extend(js_array_urls(body, "videos"));
  }
  let mut seen = HashSet::new();
  links.retain(|link| seen.insert(link.clone()));
  links
}

fn js_array_urls(body: &str, variable: &str) -> Vec<String> {
  let Some(expression) = js_array_expression(body, variable) else {
    return Vec::new();
  };
  let normalized = expression.replace("\\/", "/").replace("&amp;", "&");
  normalized
    .split(['\'', '"'])
    .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
    .filter_map(|value| {
      let url = Url::parse(value).ok()?;
      (is_public_web_url(&url) && matches!(url.scheme(), "http" | "https")).then(|| url.to_string())
    })
    .collect()
}

fn js_array_expression<'a>(body: &'a str, variable: &str) -> Option<&'a str> {
  for (index, _) in body.match_indices(variable) {
    let assignment = body.get(index + variable.len()..)?.trim_start();
    let Some(expression) = assignment.strip_prefix('=') else {
      continue;
    };
    let expression = expression.trim_start();
    if expression.starts_with('[') {
      return Some(
        expression
          .split_once(';')
          .map_or(expression, |(value, _)| value),
      );
    }
  }
  None
}

fn is_direct_mp4_url(media_url: &str) -> bool {
  Url::parse(media_url).is_ok_and(|url| url.path().to_ascii_lowercase().ends_with(".mp4"))
}

fn extract_media_urls(body: &str, page_url: &str) -> Vec<String> {
  let normalized = body
    .replace("\\/", "/")
    .replace("&amp;", "&")
    .replace("&#038;", "&");
  let Ok(page) = Url::parse(page_url) else {
    return Vec::new();
  };
  let mut urls = Vec::new();
  for candidate in normalized.split(|character: char| {
    character.is_whitespace()
      || matches!(
        character,
        '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}'
      )
  }) {
    let candidate =
      candidate.trim_matches(|character: char| matches!(character, ',' | ';' | ':' | '!'));
    let lower = candidate.to_ascii_lowercase();
    if !lower.contains(".m3u8") && !lower.contains(".mp4") {
      continue;
    }
    let candidate = candidate.trim_end_matches(['.', ',', ';']);
    let Ok(url) = page.join(candidate) else {
      continue;
    };
    let path = url.path().to_ascii_lowercase();
    if !lower.contains(".m3u8") && !path.ends_with(".mp4") {
      continue;
    }
    if matches!(url.scheme(), "http" | "https")
      && is_public_web_url(&url)
      && !urls.contains(&url.to_string())
    {
      urls.push(url.to_string());
    }
  }
  urls
}

#[cfg(test)]
#[path = "../../tests/unit/providers/site.rs"]
mod tests;
