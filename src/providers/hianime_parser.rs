use scraper::{Element, Html, Selector};

use crate::domain::{anime::Anime, episode::Episode};

pub(super) fn parse_search_results(body: &str, base_url: &str) -> Result<Vec<Anime>, &'static str> {
    let document = Html::parse_document(body);
    let selector = Selector::parse("div.film-detail h3.film-name a")
        .map_err(|_| "div.film-detail h3.film-name a")?;
    let mut results = Vec::new();

    for element in document.select(&selector) {
        let mut ancestor = element.parent_element();
        let mut in_sidebar = false;
        while let Some(parent) = ancestor {
            if parent.value().attr("id") == Some("main-sidebar") {
                in_sidebar = true;
                break;
            }
            ancestor = parent.parent_element();
        }
        if in_sidebar {
            continue;
        }

        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let Some(title) = element
            .value()
            .attr("title")
            .map(str::trim)
            .filter(|title| !title.is_empty())
        else {
            continue;
        };
        let id = href
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let url = if href.starts_with("http://") || href.starts_with("https://") {
            href.to_owned()
        } else {
            format!(
                "{}{}",
                base_url,
                if href.starts_with('/') {
                    href.to_owned()
                } else {
                    format!("/{href}")
                }
            )
        };
        if !results.iter().any(|anime: &Anime| anime.id == id) {
            results.push(Anime::new(id, title, url));
        }
    }

    Ok(results)
}

pub(super) fn parse_episodes(
    body: &str,
    base_url: &str,
    anime_id: &str,
) -> Result<Vec<Episode>, &'static str> {
    let document = Html::parse_fragment(&body.replace("\\", ""));
    let selector = Selector::parse(".ep-item").map_err(|_| ".ep-item")?;
    let mut episodes = Vec::new();

    for element in document.select(&selector) {
        let Some(number) = element
            .value()
            .attr("data-number")
            .map(str::trim)
            .filter(|number| !number.is_empty())
        else {
            continue;
        };
        let Some(id) = element.value().attr("data-id").filter(|id| !id.is_empty()) else {
            continue;
        };
        let watch_url = format!("{}/watch/{}?ep={}", base_url, anime_id, id);
        episodes.push(Episode::new(number, None, watch_url));
    }

    Ok(episodes)
}

#[cfg(test)]
mod tests {
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
}
