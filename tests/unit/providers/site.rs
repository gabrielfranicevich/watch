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
  assert_eq!(ProviderId::Gnula.language(), Language::Spanish);
  assert_eq!(ProviderId::Gnula.language_pair(), None);
  assert_eq!(ProviderId::PelisPedia.language(), Language::Spanish);
  assert_eq!(ProviderId::PelisPedia.language_pair(), None);
  assert_eq!(ProviderId::PelisPlus.language(), Language::Spanish);
  assert_eq!(ProviderId::PelisPlus.language_pair(), None);
  assert_eq!(ProviderId::HDToday.language(), Language::English);
  assert_eq!(ProviderId::HDToday.language_pair(), None);
  assert_eq!(ProviderId::Pelis7.language(), Language::Spanish);
  assert_eq!(ProviderId::Pelis7.language_pair(), None);
  assert_eq!(ProviderId::ALL.len(), 15);
  assert!(ProviderId::ALL.contains(&ProviderId::Pelis7));
  assert!(ProviderId::ALL.contains(&ProviderId::HDToday));
  assert!(ProviderId::ALL.contains(&ProviderId::PelisPlus));
  assert!(ProviderId::ALL.contains(&ProviderId::PelisPedia));
  assert_eq!(ProviderId::ACTIVE.len(), 10);
  assert_eq!(
    ProviderId::ACTIVE,
    [
      ProviderId::HiAnime,
      ProviderId::GoGoAnime,
      ProviderId::AniWatch,
      ProviderId::TioAnime,
      ProviderId::JkAnime,
      ProviderId::Gnula,
      ProviderId::PelisPedia,
      ProviderId::PelisPlus,
      ProviderId::HDToday,
      ProviderId::Pelis7,
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
fn generic_hls_resolver_sets_the_default_audio_and_subtitle_renditions() {
  use std::io::{Read, Write};
  use std::net::TcpListener;

  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = listener.local_addr().unwrap();
  let playlist = r#"#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="Japanese",LANGUAGE="ja",DEFAULT=YES,URI="audio/ja.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="audio",NAME="English",LANGUAGE="en",DEFAULT=NO,URI="audio/en.m3u8"
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="Spanish",LANGUAGE="es",DEFAULT=YES,URI="subtitles/es.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=1000000,RESOLUTION=1280x720,AUDIO="audio",SUBTITLES="subs"
video.m3u8
"#;
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
    write!(
      connection,
      "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.apple.mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{playlist}",
      playlist.len()
    )
    .unwrap();
  });

  let resolver = SiteStreamResolver::new(ProviderId::JkAnime);
  let master = format!("http://{address}/master.m3u8");
  let resolved = resolver
    .resolve_media_url(&master, &master, "best")
    .unwrap();
  server.join().unwrap();

  assert_eq!(
    resolved.stream.audio_url.as_deref(),
    Some(format!("http://{address}/audio/ja.m3u8").as_str())
  );
  assert_eq!(
    resolved
      .subtitle
      .as_ref()
      .map(|subtitle| subtitle.url.as_str()),
    Some(format!("http://{address}/subtitles/es.m3u8").as_str())
  );
  assert_eq!(resolved.language_pair.label(), "ja/es");
  assert_eq!(
    resolved.audio_language_source,
    Some(LanguageMetadataSource::HlsManifest)
  );
  assert_eq!(
    resolved.subtitle_language_source,
    Some(LanguageMetadataSource::HlsManifest)
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

#[test]
fn gnula_search_cards_report_explicit_audio_language_without_guessing_subtitles() {
  let html = r#"
    <article class="movie-card">
      <a href="/movies/idiotas/" title="Idiotas">
        <span class="quality">LAT SUB HD</span>
        <h2>Idiotas</h2>
      </a>
    </article>
  "#;
  let results = parse_search_results(html, "https://ver.gnula.one", ProviderId::Gnula, "Idiotas");
  assert_eq!(results.len(), 1);
  assert_eq!(
    results[0].language_pair,
    Some(LanguagePair::new(Some("es-419"), None))
  );
}

#[test]
fn gnula_recognizes_uppercase_accented_subtitle_labels() {
  let html = r#"
    <article class="movie-card">
      <a href="/movies/idiotas/" title="Idiotas">
        <span class="quality">LAT SUBTÍTULOS EN ESPAÑOL HD</span>
        <h2>Idiotas</h2>
      </a>
    </article>
  "#;
  let results = parse_search_results(html, "https://ver.gnula.one", ProviderId::Gnula, "Idiotas");
  assert_eq!(results.len(), 1);
  assert_eq!(
    results[0].language_pair,
    Some(LanguagePair::new(Some("es-419"), Some("es")))
  );
}

#[test]
fn gnula_cards_with_multiple_audio_options_do_not_choose_one() {
  let html = r#"
    <article class="movie-card">
      <a href="/movies/spider-man/" title="Spider-Man">
        <span class="quality">LAT CAS SUB HD</span>
        <h2>Spider-Man</h2>
      </a>
    </article>
  "#;
  let results = parse_search_results(
    html,
    "https://ver.gnula.one",
    ProviderId::Gnula,
    "Spider-Man",
  );
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].language_pair, None);
}

#[test]
fn gnula_movie_details_are_exposed_as_single_playable_entries() {
  let url = Url::parse("https://ver.gnula.one/movies/idiotas/").unwrap();
  assert!(url.path().starts_with("/movies/"));
}

#[test]
fn pelispedia_uses_verified_catalog_and_detail_paths() {
  let movies = Url::parse("https://pelispedia.mov/peliculas").unwrap();
  let series = Url::parse("https://pelispedia.mov/series").unwrap();
  let movie = Url::parse("https://pelispedia.mov/pelicula/example-film").unwrap();
  let show = Url::parse("https://pelispedia.mov/serie/example-show").unwrap();
  assert!(!ProviderId::PelisPedia.is_detail_path(&movies));
  assert!(!ProviderId::PelisPedia.is_detail_path(&series));
  assert!(ProviderId::PelisPedia.is_detail_path(&movie));
  assert!(ProviderId::PelisPedia.is_detail_path(&show));
  assert!(ProviderId::PelisPedia.is_movie_path(&movie));
  assert!(!ProviderId::PelisPedia.is_movie_path(&show));
}

#[test]
fn pelispedia_search_parser_keeps_unknown_audio_and_subtitle_languages() {
  let html = r#"
    <article>
      <a href="/serie/las-hijas-de-la-criada-Kl42uo" title="Las hijas de la criada">
        <h2>Las hijas de la criada</h2>
      </a>
    </article>
  "#;
  let results = parse_search_results(
    html,
    "https://pelispedia.mov/series",
    ProviderId::PelisPedia,
    "Las hijas de la criada",
  );
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].language_pair, None);
}

#[test]
fn pelisplus_matches_observed_movie_and_series_detail_paths() {
  let movie =
    Url::parse("https://pelisplusshd.com/peliculas/apocalipsis-z-nuevo-mundo-A1507696Bx2/")
      .unwrap();
  let show = Url::parse("https://pelisplusshd.com/serie/213375/visionquest/").unwrap();
  let episode = Url::parse("https://pelisplusshd.com/serie/213375-1-1/visionquest/").unwrap();
  assert!(ProviderId::PelisPlus.is_detail_path(&movie));
  assert!(ProviderId::PelisPlus.is_detail_path(&show));
  assert!(ProviderId::PelisPlus.is_detail_path(&episode));
  assert!(ProviderId::PelisPlus.is_movie_path(&movie));
  assert!(!ProviderId::PelisPlus.is_movie_path(&show));
}

#[test]
fn pelisplus_episode_parser_keeps_season_episode_order_and_series_identity() {
  let html = r#"
    <a href="/serie/213375-1-1/visionquest/">1x1 Sin ataduras (1x1)2026-10-13</a>
    <a href="/serie/213375-1-2/visionquest/">1x2 Teseo (1x2)2026-10-13</a>
    <a href="/serie/213375-2-1/visionquest/">2x1 Otra temporada (2x1)</a>
    <a href="/serie/999999-1-1/other-show/">1x1 unrelated</a>
    <a href="/serie/213375-1-2/visionquest/">Duplicate episode</a>
  "#;
  let episodes =
    parse_pelisplus_episodes(html, "https://pelisplusshd.com/serie/213375/visionquest/");
  assert_eq!(episodes.len(), 3);
  assert_eq!(episodes[0].number, "1001");
  assert_eq!(episodes[1].number, "1002");
  assert_eq!(episodes[2].number, "2001");
  assert!(episodes[0].title.as_deref().unwrap().contains("S1E1"));
}

#[test]
fn pelisplus_movie_pages_are_not_treated_as_series_episode_lists() {
  let html = r#"<a href="/peliculas/related-movie/">Related movie</a>"#;
  let episodes = parse_pelisplus_episodes(
    html,
    "https://pelisplusshd.com/peliculas/apocalipsis-z-nuevo-mundo-A1507696Bx2/",
  );
  assert!(episodes.is_empty());
}

#[test]
fn pelisplus_watch_now_link_is_included_as_a_player_candidate() {
  let document = Html::parse_document(r#"<a href="/serie/213375/visionquest/ver/">Watch now</a>"#);
  let links = provider_player_links(&document, "", ProviderId::PelisPlus);
  assert_eq!(links, vec!["/serie/213375/visionquest/ver/"]);
}

#[test]
fn hdtoday_matches_observed_movie_and_tv_detail_paths() {
  let movie = Url::parse("https://hdtoday24.at/movie/apocalypse-z-new-world-pEk4yidyema6").unwrap();
  let show = Url::parse("https://hdtoday24.at/tv/deep-waters-oneomCeeW6XW").unwrap();
  let player =
    Url::parse("https://hdtoday24.at/watch-movie/apocalypse-z-new-world-pEk4yidyema6").unwrap();
  assert!(ProviderId::HDToday.is_detail_path(&movie));
  assert!(ProviderId::HDToday.is_detail_path(&show));
  assert!(!ProviderId::HDToday.is_detail_path(&player));
  assert!(ProviderId::HDToday.is_movie_path(&movie));
  assert!(!ProviderId::HDToday.is_movie_path(&show));
}

#[test]
fn hdtoday_search_parser_accepts_movie_and_tv_cards() {
  let html = r#"
    <a href="/movie/apocalypse-z-new-world-pEk4yidyema6" title="Apocalypse Z: New World">Apocalypse Z: New World</a>
    <a href="/tv/deep-waters-oneomCeeW6XW" title="Deep Waters">Deep Waters</a>
  "#;
  let results = parse_search_results(
    html,
    "https://hdtoday24.at/home",
    ProviderId::HDToday,
    "Apocalypse Z",
  );
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, "Apocalypse Z: New World");
  assert_eq!(results[0].language_pair, None);
}

#[test]
fn hdtoday_watch_now_link_is_followed_to_the_player_page() {
  let document = Html::parse_document(
    r#"<a href="/watch-movie/apocalypse-z-new-world-pEk4yidyema6">Watch now</a>"#,
  );
  let links = provider_player_links(&document, "", ProviderId::HDToday);
  assert_eq!(
    links,
    vec!["/watch-movie/apocalypse-z-new-world-pEk4yidyema6"]
  );
}

#[test]
fn hdtoday_movie_details_fall_back_to_a_single_movie_entry() {
  let url = Url::parse("https://hdtoday24.at/movie/apocalypse-z-new-world-pEk4yidyema6").unwrap();
  assert!(ProviderId::HDToday.is_movie_path(&url));
}

#[test]
fn pelis7_uses_fallback_detail_path_patterns_without_assuming_audio_or_subtitles() {
  let movie = Url::parse("https://pelis7.com/peliculas/apocalipsis-z/").unwrap();
  let show = Url::parse("https://pelis7.com/series/visionquest/").unwrap();
  assert!(ProviderId::Pelis7.is_detail_path(&movie));
  assert!(ProviderId::Pelis7.is_detail_path(&show));
  assert!(ProviderId::Pelis7.is_movie_path(&movie));
  assert!(!ProviderId::Pelis7.is_movie_path(&show));
  assert_eq!(ProviderId::Pelis7.language_pair(), None);
}

#[test]
fn pelis7_search_parser_accepts_fallback_movie_and_series_patterns() {
  let html = r#"
    <a href="/peliculas/apocalipsis-z/" title="Apocalipsis Z">Apocalipsis Z</a>
    <a href="/series/visionquest/" title="VisionQuest">VisionQuest</a>
  "#;
  let results = parse_search_results(
    html,
    "https://pelis7.com/",
    ProviderId::Pelis7,
    "Apocalipsis Z",
  );
  assert_eq!(results.len(), 1);
  assert_eq!(results[0].title, "Apocalipsis Z");
  assert_eq!(results[0].language_pair, None);
}

#[test]
fn pelis7_search_routes_include_common_search_form_fallbacks() {
  let routes = ProviderId::Pelis7.search_routes();
  assert!(
    routes
      .iter()
      .any(|route| route.path == "/" && route.key == Some("s"))
  );
  assert!(
    routes
      .iter()
      .any(|route| route.path == "/buscar" && route.key == Some("q"))
  );
  assert!(
    routes
      .iter()
      .any(|route| route.path == "/search" && route.key == Some("q"))
  );
}

#[test]
fn subtitle_track_uses_srclang_instead_of_display_label() {
  let document = Html::parse_document(
    r#"<video><track src="/subtitles/es.vtt" label="Español" srclang="es"></video>"#,
  );
  let subtitle = find_subtitle(&document, "https://cdn.example/player.html").unwrap();
  assert_eq!(subtitle.language.as_deref(), Some("es"));
}

#[test]
fn subtitle_track_without_srclang_does_not_guess_from_display_label() {
  let document =
    Html::parse_document(r#"<video><track src="/subtitles/en.vtt" label="English"></video>"#);
  let subtitle = find_subtitle(&document, "https://cdn.example/player.html").unwrap();
  assert_eq!(subtitle.language, None);
}

#[test]
fn resolved_hls_language_pair_is_enriched_by_explicit_page_subtitle_language() {
  let mut media = ResolvedMedia {
    stream: Stream::new("https://cdn.example/video.m3u8", None, None),
    subtitle: None,
    mal_id: None,
    qualities: Vec::new(),
    language_pair: LanguagePair::new(Some("ja"), None),
    audio_language_source: Some(LanguageMetadataSource::HlsManifest),
    subtitle_language_source: None,
    renditions: Vec::new(),
  };
  let subtitle = Subtitle::new("https://cdn.example/subtitles/es.vtt", Some("ES".into()));
  merge_page_subtitle_metadata(&mut media, Some(&subtitle));
  assert_eq!(media.language_pair.label(), "ja/es");
  assert_eq!(
    media.subtitle_language_source,
    Some(LanguageMetadataSource::HtmlTrack)
  );
  assert_eq!(
    media
      .subtitle
      .as_ref()
      .and_then(|track| track.language.as_deref()),
    Some("ES")
  );
}

#[test]
fn explicit_manifest_subtitle_language_is_not_overwritten_by_page_track() {
  let mut media = ResolvedMedia {
    stream: Stream::new("https://cdn.example/video.m3u8", None, None),
    subtitle: None,
    mal_id: None,
    qualities: Vec::new(),
    language_pair: LanguagePair::known("ja", "en"),
    audio_language_source: Some(LanguageMetadataSource::HlsManifest),
    subtitle_language_source: Some(LanguageMetadataSource::HlsManifest),
    renditions: Vec::new(),
  };
  let subtitle = Subtitle::new("https://cdn.example/subtitles/es.vtt", Some("es".into()));
  merge_page_subtitle_metadata(&mut media, Some(&subtitle));
  assert_eq!(media.language_pair.label(), "ja/en");
  assert_eq!(
    media.subtitle_language_source,
    Some(LanguageMetadataSource::HlsManifest)
  );
}
