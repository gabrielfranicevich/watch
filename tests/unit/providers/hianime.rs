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
fn identifies_common_anti_bot_challenge_markers() {
  assert!(is_browser_verification_page(
    "<title>Just a moment...</title>"
  ));
  assert!(is_browser_verification_page(
    "<div id=\"cf-browser-verification\">"
  ));
  assert!(is_browser_verification_page("Verify you are human"));
  assert!(!is_browser_verification_page(
    "<div class=\"film-detail\">Frieren</div>"
  ));
}

#[test]
fn provider_response_distinguishes_challenge_from_empty_search_results() {
  let challenge = ensure_provider_response(
    reqwest::StatusCode::FORBIDDEN,
    "<title>Just a moment...</title>",
  )
  .unwrap_err();
  assert!(challenge.to_string().contains("anti-bot verification"));

  let forbidden =
    ensure_provider_response(reqwest::StatusCode::FORBIDDEN, "forbidden").unwrap_err();
  assert!(forbidden.to_string().contains("HTTP status 403"));

  assert!(ensure_provider_response(reqwest::StatusCode::OK, "<html></html>").is_ok());
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
#[test]
fn search_deduplicates_cards_and_ignores_incomplete_entries() {
  let body = r#"
            <div class="film-detail">
                <h3 class="film-name">
                    <a href="/watch/frieren-123" title="Frieren">First card</a>
                </h3>
            </div>
            <div class="film-detail">
                <h3 class="film-name">
                    <a href="/watch/frieren-123" title="Duplicate title">Duplicate card</a>
                </h3>
            </div>
            <div class="film-detail">
                <h3 class="film-name">
                    <a href="/watch/missing-title">No title attribute</a>
                </h3>
            </div>
            <div class="film-detail">
                <h3 class="film-name">
                    <a href="/" title="No usable identifier">Invalid card</a>
                </h3>
            </div>
        "#;
  let (base_url, server) = mock_html_response(body);
  let provider = HiAnimeProvider::with_base_url(&base_url);

  let results = provider.search("Frieren").unwrap();
  let _request = server.join().unwrap();

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].id, "frieren-123");
  assert_eq!(results[0].title, "Frieren");
}

#[test]
fn search_to_episode_listing_workflow_uses_fixture_server_only() {
  use std::io::{Read, Write};
  use std::net::TcpListener;
  use std::time::Duration;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    let mut requests = Vec::new();
    for _ in 0..2 {
      let (mut stream, _) = listener.accept().unwrap();
      stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
      let mut request = [0_u8; 4096];
      let count = stream.read(&mut request).unwrap();
      let request = String::from_utf8_lossy(&request[..count]).into_owned();
      let body = if request.starts_with("GET /search?keyword=Frieren ") {
        r#"<div class="film-detail"><h3 class="film-name"><a href="/watch/frieren-123" title="Frieren">Frieren</a></h3></div>"#
      } else if request.starts_with("GET /api/theme/episode/list/123 ") {
        r#"<a class="ep-item" data-number="1" data-id="episode-100"></a><a class="ep-item" data-number="2" data-id="episode-101"></a>"#
      } else {
        panic!("unexpected provider request: {request}");
      };
      write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
      requests.push(request);
    }
    requests
  });

  let provider = HiAnimeProvider::with_base_url(&format!("http://{address}"));
  let results = provider.search("Frieren").unwrap();
  assert_eq!(results.len(), 1);
  let episodes = provider.episodes(&results[0]).unwrap();
  let requests = server.join().unwrap();

  assert_eq!(episodes.len(), 2);
  assert_eq!(episodes[0].number, "1");
  assert_eq!(episodes[1].number, "2");
  assert!(requests[0].starts_with("GET /search?keyword=Frieren "));
  assert!(requests[1].starts_with("GET /api/theme/episode/list/123 "));
}
