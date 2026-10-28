use super::*;

#[test]
fn parses_anime_schedule_search_entries() {
  let value = serde_json::json!({
      "route": "frieren-beyond-journeys-end",
      "english": "Frieren: Beyond Journey's End",
      "romaji": "Sousou no Frieren",
      "status": "Ongoing"
  });
  let entry = parse_entry(&value).unwrap();
  assert_eq!(entry.route, "frieren-beyond-journeys-end");
  assert_eq!(entry.english_title, "Frieren: Beyond Journey's End");
  assert_eq!(entry.japanese_title, "Sousou no Frieren");
  assert_eq!(entry.status, "Ongoing");
}

#[test]
fn parses_sub_and_dub_release_data() {
  let html = r#"
            <div class="release-time-type-subs">
                <span class="release-time-episode-number">Episode 12</span>
            </div>
            <time class="countdown-time" datetime="02:30:00"></time>
            <div class="release-time-type-dub">
                <span class="release-time-episode-number">Episode 8</span>
            </div>
            <time class="countdown-time countdown-time-dub" datetime="1 day"></time>
        "#;
  assert_eq!(
    parse_release_info(html),
    ReleaseInfo {
      sub_episode: Some("12".to_owned()),
      sub_countdown: Some("02:30:00".to_owned()),
      dub_episode: Some("8".to_owned()),
      dub_countdown: Some("1 day".to_owned())
    }
  );
}

#[test]
fn allows_provider_construction_with_fixture_base_url() {
  let provider = AnimeScheduleProvider::with_base_url("http://127.0.0.1:1234/");
  assert_eq!(provider.base_url, "http://127.0.0.1:1234");
}
