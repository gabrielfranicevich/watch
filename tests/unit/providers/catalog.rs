use super::*;

fn source(provider: ProviderId, title: &str, slug: &str) -> ProviderAnime {
  ProviderAnime {
    provider,
    anime: Anime::new(slug, title, format!("{}/{}", provider.base_url(), slug)),
  }
}

#[test]
fn two_english_providers_collapse_to_one_english_label() {
  let choices = group_results(vec![
    source(ProviderId::GoGoAnime, "Dragon Ball Super: Beerus", "beerus"),
    source(ProviderId::NineAnime, "Dragon Ball Super Beerus", "beerus"),
  ]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(ja/en) Dragon Ball Super: Beerus");
  assert_eq!(choices[0].sources.len(), 2);
}

#[test]
fn two_spanish_providers_collapse_to_one_spanish_label() {
  let choices = group_results(vec![
    source(ProviderId::TioAnime, "Dragon Ball Super: Beerus", "beerus"),
    source(ProviderId::JkAnime, "Dragon Ball Super Beerus", "beerus"),
  ]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(ja/es) Dragon Ball Super: Beerus");
  assert_eq!(choices[0].sources.len(), 2);
}

#[test]
fn bilingual_title_is_listed_in_english_then_spanish_order() {
  let choices = group_results(vec![
    source(
      ProviderId::NineAnime,
      "Frieren: Beyond Journey's End",
      "frieren-en",
    ),
    source(
      ProviderId::MonosChinos,
      "Frieren: Beyond Journey's End",
      "frieren-es",
    ),
  ]);
  assert_eq!(choices.len(), 2);
  assert_eq!(choices[0].label, "(ja/en) Frieren: Beyond Journey's End");
  assert_eq!(choices[1].label, "(ja/es) Frieren: Beyond Journey's End");
  assert_eq!(choices[0].sources.len(), 1);
  assert_eq!(choices[1].sources.len(), 1);
}

#[test]
fn unambiguous_title_includes_its_language_label() {
  let choices = group_results(vec![source(ProviderId::NineAnime, "Frieren", "frieren")]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(ja/en) Frieren");
  assert_eq!(
    choices[0].sources[0].anime.language_pair_source,
    Some(LanguageMetadataSource::ProviderDefault)
  );
}

#[test]
fn episode_pages_and_episode_titled_results_are_not_listed_as_anime() {
  assert!(is_episode_search_result(
    "Episodio 1 Dragon Ball Super: Beerus 2026-10-11",
    "https://tioanime.com/ver/dragon-ball-super-beerus-1"
  ));
  assert!(is_episode_search_result(
    "Dragon Ball Super: Beerus",
    "https://jkanime.surf/dragon-ball-super-beerus-episodio-3/"
  ));
  assert!(is_episode_search_result(
    "Miss Kobayashi's Dragon Maid Episode 14: Valentine's Day",
    "https://hianime.at/watch/miss-kobayashi-episode-14"
  ));
  assert!(!is_episode_search_result(
    "Dragon Ball Z Special 1: Bardock, The Father of Goku",
    "https://hianime.at/watch/dragon-ball-z-special-1"
  ));
}

#[test]
fn strips_search_genre_and_status_prefixes_before_adding_language_labels() {
  let choices = group_results(vec![
    source(
      ProviderId::HiAnime,
      "Dragon Ball Super: Beerus",
      "beerus-en",
    ),
    source(
      ProviderId::TioAnime,
      "Dragon Ball Super: Beerus Acción",
      "beerus-es",
    ),
    source(
      ProviderId::JkAnime,
      "Serie En emision Dragon Ball Super: Beerus",
      "beerus-jk",
    ),
  ]);
  assert_eq!(choices.len(), 2);
  assert_eq!(choices[0].label, "(ja/en) Dragon Ball Super: Beerus");
  assert_eq!(choices[1].label, "(ja/es) Dragon Ball Super: Beerus");
  assert_eq!(choices[1].sources.len(), 2);
}

#[test]
fn grouping_discards_duplicate_cards_from_the_same_provider() {
  let choices = group_results(vec![
    source(ProviderId::NineAnime, "Frieren", "frieren"),
    source(ProviderId::NineAnime, "Frieren", "frieren-copy"),
    source(ProviderId::GoGoAnime, "Frieren", "frieren-two"),
  ]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(ja/en) Frieren");
  assert_eq!(choices[0].sources.len(), 2);
}

#[test]
fn search_choice_order_does_not_depend_on_provider_completion_order() {
  let mut sources = vec![
    source(ProviderId::JkAnime, "Beerus", "jkanime"),
    source(ProviderId::HiAnime, "Beerus", "hianime"),
    source(ProviderId::MonosChinos, "Beerus", "monoschinos"),
    source(ProviderId::TioAnime, "Beerus", "tioanime"),
  ];
  sort_sources_by_provider_order(&mut sources);
  assert_eq!(
    sources
      .iter()
      .map(|source| source.provider)
      .collect::<Vec<_>>(),
    vec![
      ProviderId::HiAnime,
      ProviderId::TioAnime,
      ProviderId::JkAnime,
      ProviderId::MonosChinos,
    ]
  );
}

#[test]
fn normalizes_punctuation_and_common_language_suffixes() {
  assert_eq!(
    normalize_provider_title("My Hero Academia: English Subbed"),
    normalize_provider_title("My Hero Academia")
  );
  assert_eq!(
    normalize_provider_title("Frieren: Beyond Journey's End"),
    normalize_provider_title("Frieren Beyond Journey s End")
  );
}

#[test]
fn explicit_source_language_pairs_override_provider_defaults() {
  let mut source = source(ProviderId::TioAnime, "Frieren", "frieren");
  source.anime.language_pair = Some(LanguagePair::known("ja", "en"));
  let choices = group_results(vec![source]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(ja/en) Frieren");
  assert_eq!(
    choices[0].sources[0].anime.language_pair_source,
    Some(LanguageMetadataSource::SearchMetadata)
  );
}

#[test]
fn provider_defaults_are_marked_as_provider_default_evidence() {
  let choices = group_results(vec![source(ProviderId::TioAnime, "Frieren", "frieren")]);
  assert_eq!(choices.len(), 1);
  assert_eq!(
    choices[0].sources[0].anime.language_pair_source,
    Some(LanguageMetadataSource::ProviderDefault)
  );
}

#[test]
fn unknown_language_pairs_are_not_assumed_to_be_english_or_spanish() {
  let mut source = source(ProviderId::TioAnime, "Frieren", "frieren");
  source.anime.language_pair = Some(LanguagePair::new(None, None));
  let choices = group_results(vec![source]);
  assert_eq!(choices.len(), 1);
  assert_eq!(choices[0].label, "(?/?) Frieren");
}

#[test]
fn same_title_with_different_audio_subtitle_pairs_stays_separate() {
  let mut japanese_english = source(ProviderId::TioAnime, "Frieren", "frieren-ja-en");
  japanese_english.anime.language_pair = Some(LanguagePair::known("ja", "en"));
  let mut japanese_spanish = source(ProviderId::TioAnime, "Frieren", "frieren-ja-es");
  japanese_spanish.anime.language_pair = Some(LanguagePair::known("ja", "es"));
  let choices = group_results(vec![japanese_english, japanese_spanish]);
  assert_eq!(choices.len(), 2);
  assert_eq!(choices[0].label, "(ja/en) Frieren");
  assert_eq!(choices[1].label, "(ja/es) Frieren");
}
