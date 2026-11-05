use super::*;

#[test]
fn stores_provider_identity_and_title() {
  let anime = Anime::new("frieren", "Frieren", "https://example.test/anime/frieren");
  assert_eq!(anime.id, "frieren");
  assert_eq!(anime.title, "Frieren");
  assert_eq!(anime.language_pair_source, None);
}
