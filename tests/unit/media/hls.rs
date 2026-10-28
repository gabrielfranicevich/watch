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
  let playlist = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"English\",DEFAULT=YES,URI=\"../audio/en.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360.m3u8\n";
  assert_eq!(
    parse_default_audio_url(playlist, "https://cdn.example/hls/master.m3u8").unwrap(),
    Some("https://cdn.example/audio/en.m3u8".to_owned())
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
