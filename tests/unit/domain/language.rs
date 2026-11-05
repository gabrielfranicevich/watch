use super::*;

#[test]
fn normalizes_audio_and_subtitle_language_codes() {
  let pair = LanguagePair::new(Some(" JA "), Some("ES"));
  assert_eq!(pair.audio.as_deref(), Some("ja"));
  assert_eq!(pair.subtitles.as_deref(), Some("es"));
  assert_eq!(pair.label(), "ja/es");
}

#[test]
fn keeps_each_unknown_language_independent() {
  let audio_unknown = LanguagePair::new(None, Some("en"));
  let subtitles_unknown = LanguagePair::new(Some("ja"), None);
  let both_unknown = LanguagePair::new(None, None);
  assert_eq!(audio_unknown.label(), "?/en");
  assert_eq!(subtitles_unknown.label(), "ja/?");
  assert_eq!(both_unknown.label(), "?/?");
}

#[test]
fn normalizes_language_names_and_region_codes() {
  assert_eq!(
    LanguagePair::new(Some("English"), Some("Español")).label(),
    "en/es"
  );
  assert_eq!(
    LanguagePair::new(Some("JA-jp"), Some("es-419")).label(),
    "ja-jp/es-419"
  );
}

#[test]
fn treats_unknown_language_names_as_unknown_codes_without_guessing() {
  assert_eq!(
    LanguagePair::new(Some("unknown language"), Some("")).label(),
    "unknown language/?"
  );
}
