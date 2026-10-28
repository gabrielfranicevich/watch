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

fn mock_http_response(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<()>) {
  use std::io::Write;
  use std::net::TcpListener;
  use std::time::Duration;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let handle = std::thread::spawn(move || {
    let (mut connection, _) = listener.accept().unwrap();
    connection
      .set_read_timeout(Some(Duration::from_secs(2)))
      .unwrap();
    read_request_headers(&mut connection);
    write!(
      connection,
      "HTTP/1.1 {status} Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    )
    .unwrap();
    connection.flush().unwrap();
  });
  (format!("http://{address}/error"), handle)
}

fn read_request_headers(connection: &mut std::net::TcpStream) {
  use std::io::Read;

  let mut request = Vec::new();
  let mut buffer = [0_u8; 1024];
  while request.len() < 16 * 1024 && !request.windows(4).any(|part| part == b"\r\n\r\n") {
    let Ok(count) = connection.read(&mut buffer) else {
      break;
    };
    if count == 0 {
      break;
    }
    request.extend_from_slice(&buffer[..count]);
  }
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
    read_request_headers(&mut connection);
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

  let (stream, server) = mock_http_response(403, "forbidden");
  let response = reqwest::blocking::Client::new()
    .get(&stream)
    .send()
    .unwrap();
  let error = provider_response_body(response).unwrap_err();
  server.join().unwrap();
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
