use reqwest::blocking::Client;
use std::time::Duration;

use crate::domain::{anime::Anime, episode::Episode};

const BASE_URL: &str = "https://hianime.at";

#[derive(Debug)]
pub enum ProviderError {
  Request(reqwest::Error),
  InvalidSelector(&'static str),
  UnexpectedResponse(String),
}

impl std::fmt::Display for ProviderError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Request(error) => write!(f, "provider request failed: {error}"),
      Self::InvalidSelector(selector) => write!(f, "invalid HTML selector: {selector}"),
      Self::UnexpectedResponse(message) => f.write_str(message),
    }
  }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
  fn from(error: reqwest::Error) -> Self {
    Self::Request(error)
  }
}

#[derive(Debug, Clone)]
pub struct HiAnimeProvider {
  client: Client,
  base_url: String,
}

impl Default for HiAnimeProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl HiAnimeProvider {
  pub fn new() -> Self {
    let client = Client::builder()
      .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36")
      .timeout(Duration::from_secs(15))
      .build()
      .expect("static HTTP client configuration is valid");
    Self {
      client,
      base_url: BASE_URL.to_owned(),
    }
  }

  #[cfg(test)]
  fn with_base_url(base_url: &str) -> Self {
    Self {
      client: Client::new(),
      base_url: base_url.trim_end_matches('/').to_owned(),
    }
  }

  pub fn search(&self, query: &str) -> Result<Vec<Anime>, ProviderError> {
    let url = format!("{}/search", self.base_url);
    let response = self.client.get(url).query(&[("keyword", query)]).send()?;
    let status = response.status();
    let body = response.text()?;
    ensure_provider_response(status, &body)?;
    super::hianime_parser::parse_search_results(&body, &self.base_url)
      .map_err(ProviderError::InvalidSelector)
  }

  pub fn episodes(&self, anime: &Anime) -> Result<Vec<Episode>, ProviderError> {
    let slug = anime.id.rsplit('-').next().unwrap_or(&anime.id);
    let url = format!("{}/api/theme/episode/list/{}", self.base_url, slug);
    let response = self.client.get(url).send()?;
    let status = response.status();
    let body = response.text()?;
    ensure_provider_response(status, &body)?;
    super::hianime_parser::parse_episodes(&body, &self.base_url, &anime.id)
      .map_err(ProviderError::InvalidSelector)
  }
}

pub(crate) fn is_browser_verification_page(body: &str) -> bool {
  let normalized = body.to_ascii_lowercase();
  [
    "just a moment",
    "cf-browser-verification",
    "challenge-platform",
    "verify you are human",
  ]
  .iter()
  .any(|marker| normalized.contains(marker))
}

fn ensure_provider_response(status: reqwest::StatusCode, body: &str) -> Result<(), ProviderError> {
  if is_browser_verification_page(body) {
    return Err(ProviderError::UnexpectedResponse(
            "HiAnime returned an anti-bot verification page; retry later or use a compatible curl-impersonate setup".into(),
        ));
  }
  if !status.is_success() {
    return Err(ProviderError::UnexpectedResponse(format!(
      "HiAnime returned HTTP status {status}"
    )));
  }
  Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/providers/hianime.rs"]
mod tests;
