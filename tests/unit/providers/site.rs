use super::*;

#[test]
fn empty_search_query_never_matches_every_title() {
  assert!(!matches_query("Frieren: Beyond Journey's End", ""));
  assert!(!matches_query("Frieren: Beyond Journey's End", "   "));
}

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
  assert_eq!(ProviderId::ACTIVE.len(), 5);
  assert_eq!(
    ProviderId::ACTIVE,
    [
      ProviderId::HiAnime,
      ProviderId::GoGoAnime,
      ProviderId::AniWatch,
      ProviderId::TioAnime,
      ProviderId::JkAnime,
    ]
  );
}

#[test]
fn search_parser_ignores_navigation_and_deduplicates_provider_cards() {
  let html = r#"<nav><a href="/series/">Search</a></nav>
            <div><a href="/series/frieren/" title="Frieren: Beyond Journey's End">Frieren</a>
            <a href="/series/frieren/" title="Duplicate">Frieren</a></div>"#;
  let results = parse_search_results(
    html,
    "https://gogoanime.by",
    ProviderId::GoGoAnime,
    "Frieren",
  );
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, "Frieren: Beyond Journey's End");
}

#[test]
fn search_fetches_supported_routes_and_merges_results_from_each() {
  use std::io::{Read, Write};
  use std::net::TcpListener;
  use std::time::Duration;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    let mut requested_paths = Vec::new();
    for _ in 0..2 {
      let (mut connection, _) = listener.accept().unwrap();
      connection
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
      let mut request = Vec::new();
      let mut buffer = [0_u8; 2048];
      while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        let count = connection.read(&mut buffer).unwrap();
        if count == 0 {
          break;
        }
        request.extend_from_slice(&buffer[..count]);
      }
      let request = String::from_utf8_lossy(&request).into_owned();
      let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_owned();
      let body = if path.starts_with("/search.html?keyword=Frieren") {
        r#"<a href="/series/frieren-one" title="Frieren One">Frieren One</a>"#
      } else if path.starts_with("/?s=Frieren") {
        r#"<a href="/series/frieren-two" title="Frieren Two">Frieren Two</a>"#
      } else {
        panic!("unexpected search route: {path}");
      };
      write!(
          connection,
          "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
          body.len(),
          body
        )
        .unwrap();
      requested_paths.push(path);
    }
    requested_paths
  });

  let provider = SiteProvider {
    id: ProviderId::GoGoAnime,
    client: Client::builder()
      .timeout(Duration::from_secs(2))
      .build()
      .unwrap(),
    base_url: format!("http://{address}"),
  };
  let results = provider.search("Frieren").unwrap();
  let requested_paths = server.join().unwrap();

  assert_eq!(requested_paths.len(), 2);
  assert_eq!(results.len(), 2);
  assert!(results.iter().any(|anime| anime.title == "Frieren One"));
  assert!(results.iter().any(|anime| anime.title == "Frieren Two"));
}

#[test]
fn episode_parser_sorts_numeric_episodes_and_skips_navigation() {
  let html = r#"<a href="/watch/frieren/ep-10">Episode 10</a>
            <a href="/watch/frieren/ep-2">Episode 2</a>
            <a href="/watch/frieren/ep-2?server=mirror">Episode 2</a>
            <a href="/watch/frieren/ep-1">Episode 1</a><a href="/about">About</a>"#;
  let episodes = parse_episode_links(
    html,
    "https://animekai.be",
    "https://animekai.be/watch/frieren",
  );
  assert_eq!(episodes.len(), 3);
  assert_eq!(episodes[0].number, "1");
  assert_eq!(episodes[1].number, "2");
  assert_eq!(episodes[2].number, "10");
  assert!(episodes[1].url.ends_with("/watch/frieren/ep-2"));
}

#[test]
fn episode_parser_rejects_episode_links_for_other_anime_and_navigation() {
  let html = r#"
      <a href="/series/frieren-episode-1">Episode 1</a>
      <a href="/series/other-anime-episode-17">Episode 17</a>
      <a href="/series/frieren/recommended">Episode 12</a>
      <a href="/series/frieren-episode-2">Episode 2</a>
    "#;
  let episodes = parse_episode_links(
    html,
    "https://gogoanime.by",
    "https://gogoanime.by/series/frieren/",
  );
  assert_eq!(
    episodes
      .iter()
      .map(|episode| episode.number.as_str())
      .collect::<Vec<_>>(),
    vec!["1", "2"]
  );
}

#[test]
fn monos_chinos_episode_links_match_series_slug_without_sub_espanol_suffix() {
  let html = r#"<a href="/ver/doragon-boru-supa-birusu-episodio-3">EP 3</a>"#;
  let episodes = parse_episode_links(
    html,
    "https://monoschinos.st/anime/doragon-boru-supa-birusu-sub-espanol",
    "https://monoschinos.st/anime/doragon-boru-supa-birusu-sub-espanol",
  );
  assert_eq!(episodes.len(), 1);
  assert_eq!(episodes[0].number, "3");
  assert_eq!(
    episodes[0].url,
    "https://monoschinos.st/ver/doragon-boru-supa-birusu-episodio-3"
  );
}

#[test]
fn jkanime_episode_links_recognize_the_episodio_path_suffix() {
  let html =
    r#"<a href="/dragon-ball-super-beerus-episodio-3/">Episodio 3 Dragon Ball Super: Beerus</a>"#;
  let anime_url = "https://jkanime.surf/anime/dragon-ball-super-beerus/";
  let episodes = parse_episode_links(html, anime_url, anime_url);
  assert_eq!(episodes.len(), 1);
  assert_eq!(episodes[0].number, "3");
  assert_eq!(
    episodes[0].url,
    "https://jkanime.surf/dragon-ball-super-beerus-episodio-3/"
  );
}

#[test]
fn monos_chinos_resolver_follows_only_its_explicit_direct_player_link() {
  let html = Html::parse_document(
    r#"<a class="direct-link" href="https://savefiles.com/embed-example.html">Servidor</a>
         <a href="https://unrelated.example/">Recomendación</a>"#,
  );
  assert_eq!(
    embedded_player_links(&html, ProviderId::MonosChinos),
    vec!["https://savefiles.com/embed-example.html"]
  );
  assert!(embedded_player_links(&html, ProviderId::TioAnime).is_empty());
}

#[test]
fn tioanime_player_links_are_read_from_its_videos_array() {
  let body = r#"var videos = [["Mega","https:\/\/mega.nz\/embed\/example"],["Voe","https:\/\/voe.sx\/e\/example",0,0]];"#;
  let document = Html::parse_document("<html></html>");
  assert_eq!(
    provider_player_links(&document, body, ProviderId::TioAnime),
    vec!["https://mega.nz/embed/example", "https://voe.sx/e/example",]
  );
}

#[test]
fn rejects_hls_variant_endpoints_that_return_html() {
  use std::io::{Read, Write};
  use std::net::TcpListener;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    for (path, body, content_type) in [
      (
        "/master",
        "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000,RESOLUTION=640x360\n/player\n",
        "application/vnd.apple.mpegurl",
      ),
      ("/player", "<html>embed page</html>", "text/html"),
    ] {
      let (mut connection, _) = listener.accept().unwrap();
      let mut request = Vec::new();
      let mut buffer = [0_u8; 1024];
      while !request.windows(4).any(|part| part == b"\r\n\r\n") {
        let count = connection.read(&mut buffer).unwrap();
        if count == 0 {
          break;
        }
        request.extend_from_slice(&buffer[..count]);
      }
      let body = if String::from_utf8_lossy(&request).contains(path) {
        body
      } else {
        "unexpected request"
      };
      write!(
          connection,
          "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
          body.len()
        )
        .unwrap();
    }
  });

  let resolver = SiteStreamResolver::new(ProviderId::JkAnime);
  let master = format!("http://{address}/master");
  let error = resolver
    .resolve_media_url(&master, &master, "best")
    .unwrap_err()
    .to_string();
  server.join().unwrap();

  assert!(error.contains("HTML instead of media"), "{error}");
}

#[test]
fn rejects_extensionless_media_candidates_that_return_html() {
  use std::io::{Read, Write};
  use std::net::TcpListener;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    let (mut connection, _) = listener.accept().unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
      let count = connection.read(&mut buffer).unwrap();
      if count == 0 {
        break;
      }
      request.extend_from_slice(&buffer[..count]);
    }
    let body = "<html>embed player</html>";
    write!(
        connection,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      )
      .unwrap();
  });

  let resolver = SiteStreamResolver::new(ProviderId::JkAnime);
  let url = format!("http://{address}/extensionless-player");
  let error = resolver
    .resolve_media_url(&url, "http://jkanime.surf/", "best")
    .unwrap_err()
    .to_string();
  server.join().unwrap();

  assert!(error.contains("returned HTML instead of a media playlist"));
}

#[test]
fn rejects_direct_mp4_candidates_that_return_404() {
  use std::io::{Read, Write};
  use std::net::TcpListener;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    let (mut connection, _) = listener.accept().unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
      let count = connection.read(&mut buffer).unwrap();
      if count == 0 {
        break;
      }
      request.extend_from_slice(&buffer[..count]);
    }
    let body = "missing";
    write!(
        connection,
        "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      )
      .unwrap();
  });

  let resolver = SiteStreamResolver::new(ProviderId::TioAnime);
  let url = format!("http://{address}/missing.mp4");
  let error = resolver
    .resolve_media_url(&url, "http://tioanime.com/", "best")
    .unwrap_err()
    .to_string();
  server.join().unwrap();

  assert!(error.contains("HTTP status 404"), "{error}");
}

#[test]
fn mp4upload_host_is_not_mistaken_for_an_mp4_file() {
  assert!(is_direct_mp4_url("https://cdn.example/video.mp4?token=abc"));
  assert!(!is_direct_mp4_url("https://www.mp4upload.com/znd9u99pnybq"));
}

#[test]
fn extracts_tioanime_next_episode_date_from_empty_schedule() {
  assert_eq!(
    next_episode_date("<p>Proximo episodio: 2026-11-01</p>"),
    Some("2026-11-01".to_owned())
  );
  assert_eq!(next_episode_date("<p>Listado de episodios</p>"), None);
}

#[test]
fn parses_tioanime_episode_numbers_from_the_page_script_data() {
  let body = r#"
      <script>
        var anime_info = ["4549","dragon-ball-super-beerus","Dragon Ball Super: Beerus","2026-11-01"];
        var episodes = [3,2,1];
        var episodes_details = ["Hace 3 dias","Hace 10 dias","Hace 17 dias"];
      </script>
    "#;
  let episodes =
    parse_tioanime_episodes(body, "https://tioanime.com/anime/dragon-ball-super-beerus");
  assert_eq!(
    episodes
      .iter()
      .map(|episode| (episode.number.as_str(), episode.url.as_str()))
      .collect::<Vec<_>>(),
    vec![
      ("1", "https://tioanime.com/ver/dragon-ball-super-beerus-1"),
      ("2", "https://tioanime.com/ver/dragon-ball-super-beerus-2"),
      ("3", "https://tioanime.com/ver/dragon-ball-super-beerus-3"),
    ]
  );
}

#[test]
fn tioanime_empty_schedule_reports_the_next_episode_date() {
  use std::io::{Read, Write};
  use std::net::TcpListener;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let server = std::thread::spawn(move || {
    let (mut connection, _) = listener.accept().unwrap();
    let mut request = [0_u8; 1024];
    let _ = connection.read(&mut request);
    let body = "<h2>Listado de episodios</h2><p>Proximo episodio: 2026-11-01</p>";
    write!(
        connection,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      )
      .unwrap();
  });
  let anime = Anime::new(
    "dragon-ball-super-beerus",
    "Dragon Ball Super: Beerus",
    format!("http://{address}/anime/dragon-ball-super-beerus"),
  );
  let provider = SiteProvider {
    id: ProviderId::TioAnime,
    client: Client::new(),
    base_url: format!("http://{address}"),
  };

  let error = provider.episodes(&anime).unwrap_err().to_string();
  server.join().unwrap();
  assert!(error.contains("has no released episodes"));
  assert!(error.contains("2026-11-01"));
}

#[test]
fn jkanime_accepts_current_anime_detail_path() {
  let url = Url::parse("https://jkanime.surf/anime/dragon-ball-super-beerus/").unwrap();
  assert!(ProviderId::JkAnime.is_detail_path(&url));
}

#[test]
fn extracts_escaped_media_urls_and_preserves_query_strings() {
  let found = extract_media_urls(
    r#"<script>var src = "https:\/\/cdn.example\/video\/master.m3u8?token=abc&amp;x=1";</script>"#,
    "https://example.test/watch/episode",
  );
  assert_eq!(
    found,
    vec!["https://cdn.example/video/master.m3u8?token=abc&x=1"]
  );
}

#[test]
fn does_not_treat_mp4upload_host_as_a_direct_video_url() {
  assert!(
    extract_media_urls(
      "https://www.mp4upload.com/znd9u99pnybq",
      "https://jkanime.surf/episode/"
    )
    .is_empty()
  );
}

#[test]
fn does_not_follow_private_or_local_embed_urls() {
  assert!(!is_public_web_url(
    &Url::parse("http://127.0.0.1/player").unwrap()
  ));
  assert!(!is_public_web_url(
    &Url::parse("http://192.168.1.1/player").unwrap()
  ));
  assert!(is_public_web_url(
    &Url::parse("https://player.example/watch").unwrap()
  ));
}
