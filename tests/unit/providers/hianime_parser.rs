use super::*;

#[test]
fn parses_search_cards_and_deduplicates_ids_without_network() {
  let html = r#"
            <div class="film-detail"><h3 class="film-name">
                <a href="/watch/frieren-123" title="Frieren">Frieren</a>
            </h3></div>
            <div class="film-detail"><h3 class="film-name">
                <a href="/watch/frieren-123" title="Duplicate">Duplicate</a>
            </h3></div>
            <div class="film-detail"><h3 class="film-name">
                <a href="/" title="Missing ID">Missing ID</a>
            </h3></div>
        "#;

  let results = parse_search_results(html, "https://hianime.at").unwrap();

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].id, "frieren-123");
  assert_eq!(results[0].url, "https://hianime.at/watch/frieren-123");
}

#[test]
fn parses_episode_ids_and_skips_incomplete_items_without_network() {
  let html = r#"
            <a class="ep-item" data-number="1" data-id="episode-100"></a>
            <a class="ep-item" data-number="2"></a>
            <a class="ep-item" data-id="episode-102"></a>
        "#;

  let episodes = parse_episodes(html, "https://hianime.at", "frieren-123").unwrap();

  assert_eq!(episodes.len(), 1);
  assert_eq!(episodes[0].number, "1");
  assert_eq!(
    episodes[0].url,
    "https://hianime.at/watch/frieren-123?ep=episode-100"
  );
}
