use reqwest::blocking::Client;
use scraper::{Html, Selector};
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
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36",
            )
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
        let response = self
            .client
            .get(url)
            .query(&[("keyword", query)])
            .send()?
            .error_for_status()?;
        let document = Html::parse_document(&response.text()?);
        let selector = Selector::parse("div.film-detail h3.film-name a")
            .map_err(|_| ProviderError::InvalidSelector("div.film-detail h3.film-name a"))?;
        let mut results = Vec::new();

        for element in document.select(&selector) {
            let mut ancestor = element.parent_element();
            let mut in_sidebar = false;
            while let Some(parent) = ancestor {
                if parent.value().attr("id") == Some("main-sidebar") {
                    in_sidebar = true;
                    break;
                }
                ancestor = parent.parent_element();
            }
            if in_sidebar {
                continue;
            }

            let Some(href) = element.value().attr("href") else {
                continue;
            };
            let Some(title) = element
                .value()
                .attr("title")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let id = href
                .trim_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            let url = if href.starts_with("http://") || href.starts_with("https://") {
                href.to_owned()
            } else {
                format!(
                    "{}{}",
                    self.base_url,
                    if href.starts_with('/') {
                        href.to_owned()
                    } else {
                        format!("/{href}")
                    }
                )
            };
            if !results.iter().any(|anime: &Anime| anime.id == id) {
                results.push(Anime::new(id, title, url));
            }
        }
        Ok(results)
    }

    pub fn episodes(&self, anime: &Anime) -> Result<Vec<Episode>, ProviderError> {
        let slug = anime.id.rsplit('-').next().unwrap_or(&anime.id);
        let url = format!("{}/api/theme/episode/list/{}", self.base_url, slug);
        let response = self.client.get(url).send()?.error_for_status()?;
        let body = response.text()?;
        let document = Html::parse_fragment(&body.replace("\\", ""));
        let selector =
            Selector::parse(".ep-item").map_err(|_| ProviderError::InvalidSelector(".ep-item"))?;
        let mut episodes = Vec::new();

        for element in document.select(&selector) {
            let Some(number) = element
                .value()
                .attr("data-number")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let Some(id) = element.value().attr("data-id").filter(|s| !s.is_empty()) else {
                continue;
            };
            let watch_url = format!("{}/watch/{}?ep={}", self.base_url, anime.id, id);
            episodes.push(Episode::new(number, None, watch_url));
        }

        if episodes.is_empty() && body.contains("Just a moment") {
            return Err(ProviderError::UnexpectedResponse(
                "HiAnime returned a browser-verification page; try a curl-impersonate build or retry later".into(),
            ));
        }
        Ok(episodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_html_response(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]).into_owned();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn search_parses_fixture_and_encodes_query() {
        let body = r#"
            <div id="main-content">
                <div class="film-detail">
                    <h3 class="film-name">
                        <a href="/watch/frieren-123" title="Frieren: Beyond Journey's End">Frieren</a>
                    </h3>
                </div>
            </div>
            <div id="main-sidebar">
                <div class="film-detail">
                    <h3 class="film-name">
                        <a href="/watch/sidebar-pick-999" title="Sidebar recommendation">Sidebar pick</a>
                    </h3>
                </div>
            </div>
        "#;
        let (base_url, server) = mock_html_response(body);
        let provider = HiAnimeProvider::with_base_url(&base_url);
        let results = provider.search("Frieren: Beyond Journey's End").unwrap();
        let request = server.join().unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "frieren-123");
        assert_eq!(results[0].title, "Frieren: Beyond Journey's End");
        assert_eq!(results[0].url, format!("{base_url}/watch/frieren-123"));
        assert!(!results.iter().any(|result| result.id == "sidebar-pick-999"));
        assert!(request.starts_with("GET /search?keyword="));
        assert!(request.contains("Frieren"));
    }

    #[test]
    fn episodes_parses_fixture_and_preserves_watch_identifier() {
        let body = r#"
            <div class="ep-item" data-number="1" data-id="1001"></div>
            <div class="ep-item" data-number="2" data-id="1002"></div>
        "#;
        let (base_url, server) = mock_html_response(body);
        let provider = HiAnimeProvider::with_base_url(&base_url);
        let anime = Anime::new(
            "frieren-123",
            "Frieren",
            format!("{base_url}/watch/frieren-123"),
        );
        let episodes = provider.episodes(&anime).unwrap();
        let request = server.join().unwrap();

        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].number, "1");
        assert!(episodes[0].url.ends_with("/watch/frieren-123?ep=1001"));
        assert!(request.starts_with("GET /api/theme/episode/list/123 "));
    }

    #[test]
    fn provider_has_a_stable_default_base_url() {
        let provider = HiAnimeProvider::default();
        assert_eq!(provider.base_url, BASE_URL);
    }

    #[test]
    fn test_provider_can_use_local_fixture_server_url() {
        let provider = HiAnimeProvider::with_base_url("http://127.0.0.1:1234/");
        assert_eq!(provider.base_url, "http://127.0.0.1:1234");
    }
}
