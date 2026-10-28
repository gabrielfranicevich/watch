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
#[path = "../../tests/unit/providers/hianime_parser.rs"]
mod tests;
