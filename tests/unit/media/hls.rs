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
  let variants = parse_master_playlist(PLAYLIST, "https://cdn.example/hls/master.m3u8").unwrap();
  assert_eq!(
    select_quality(&variants, "best")
      .unwrap()
      .resolution
      .as_deref(),
    Some("1920x1080")
  );
  assert_eq!(
    select_quality(&variants, "worst")
      .unwrap()
      .resolution
      .as_deref(),
    Some("640x360")
  );
  assert_eq!(select_quality(&variants, "720p"), None);
  assert_eq!(
    select_quality(&variants, "1080p").unwrap().bandwidth,
    2500000
  );
}

#[test]
fn parses_default_audio_rendition_and_resolves_its_url() {
  let playlist = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=NO,URI=\"../audio/en.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"Japanese\",LANGUAGE=\"ja\",DEFAULT=YES,URI=\"../audio/ja.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360.m3u8\n";
  assert_eq!(
    parse_default_audio_url(playlist, "https://cdn.example/hls/master.m3u8").unwrap(),
    Some("https://cdn.example/audio/ja.m3u8".to_owned())
  );
}

#[test]
fn returns_none_when_no_default_audio_rendition_is_declared() {
  assert_eq!(
    parse_default_audio_url(
      "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\n360.m3u8\n",
      "https://cdn.example/master.m3u8"
    )
    .unwrap(),
    None
  );
}

#[test]
fn rejects_non_playlist_response() {
  assert!(parse_master_playlist("not a playlist", "https://cdn.example/master").is_err());
}

#[test]
fn detects_default_audio_and_subtitle_languages_independently() {
  let playlist = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",LANGUAGE=\"ja\",DEFAULT=YES,URI=\"audio/ja.m3u8\"\n#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",LANGUAGE=\"es\",DEFAULT=YES,URI=\"subs/es.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360.m3u8\n";
  let pair = parse_default_language_pair(playlist, "https://cdn.example/master.m3u8").unwrap();
  assert_eq!(pair.audio.as_deref(), Some("ja"));
  assert_eq!(pair.subtitles.as_deref(), Some("es"));
  assert_eq!(pair.label(), "ja/es");
}

#[test]
fn ignores_non_default_renditions_when_detecting_languages() {
  let playlist = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",LANGUAGE=\"ja\",DEFAULT=NO,URI=\"audio/ja.m3u8\"\n#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",LANGUAGE=\"en\",DEFAULT=NO,URI=\"subs/en.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360.m3u8\n";
  let pair = parse_default_language_pair(playlist, "https://cdn.example/master.m3u8").unwrap();
  assert_eq!(pair.audio, None);
  assert_eq!(pair.subtitles, None);
  assert_eq!(pair.label(), "?/?");
}

#[test]
fn rejects_language_detection_from_non_playlist_responses() {
  assert!(
    parse_default_language_pair("not a playlist", "https://cdn.example/master.m3u8").is_err()
  );
}

#[test]
fn parses_default_and_alternative_audio_and_subtitle_renditions() {
  let playlist = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"Japanese\",LANGUAGE=\"ja\",DEFAULT=YES,URI=\"audio/ja.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=NO,URI=\"audio/en.m3u8\"\n#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"Spanish\",LANGUAGE=\"es-419\",DEFAULT=NO,URI=\"subs/es.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nvideo.m3u8\n";
  let renditions = parse_renditions(playlist, "https://cdn.example/master.m3u8").unwrap();

  assert_eq!(renditions.len(), 3);
  assert_eq!(renditions[0].language.as_deref(), Some("ja"));
  assert!(renditions[0].is_default);
  assert_eq!(
    renditions[0].url.as_deref(),
    Some("https://cdn.example/audio/ja.m3u8")
  );
  assert_eq!(renditions[1].language.as_deref(), Some("en"));
  assert!(!renditions[1].is_default);
  assert_eq!(renditions[2].language.as_deref(), Some("es-419"));
  assert_eq!(renditions[2].kind, HlsRenditionType::Subtitles);
}
