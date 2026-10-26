use reqwest::blocking::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::error::Error;
use std::time::Duration;

const BASE_URL: &str = "https://animeschedule.net";

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnimeScheduleEntry {
    route: String,
    english_title: String,
    japanese_title: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseInfo {
    sub_episode: Option<String>,
    sub_countdown: Option<String>,
    dub_episode: Option<String>,
    dub_countdown: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnimeScheduleProvider {
    client: Client,
    base_url: String,
}

impl Default for AnimeScheduleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimeScheduleProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/124.0 Safari/537.36",
            )
            .timeout(Duration::from_secs(15))
            .build()
            .expect("static HTTP client configuration is valid");
        Self {
            client,
            base_url: BASE_URL.to_owned(),
        }
    }

    #[cfg(test)]
    fn with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
        }
    }

    fn search(&self, query: &str) -> Result<Vec<AnimeScheduleEntry>, Box<dyn Error>> {
        let response = self
            .client
            .get(format!("{}/api/v3/anime", self.base_url))
            .query(&[("q", query)])
            .send()?
            .error_for_status()?;
        let body: Value = response.json()?;
        let entries = body
            .as_array()
            .or_else(|| body.get("data").and_then(Value::as_array))
            .or_else(|| body.get("anime").and_then(Value::as_array))
            .ok_or("AnimeSchedule returned an unexpected search response")?;

        Ok(entries.iter().filter_map(parse_entry).collect())
    }

    pub fn show_countdown(&self, query: &str) -> Result<(), Box<dyn Error>> {
        let entries = self.search(query)?;
        if entries.is_empty() {
            return Err(format!("AnimeSchedule returned no results for '{query}'").into());
        }

        for entry in entries {
            println!("Eng: {}", entry.english_title);
            println!("Jpn: {}", entry.japanese_title);
            println!("Status: {}", entry.status);

            if !entry.status.eq_ignore_ascii_case("Finished") {
                let response = self
                    .client
                    .get(format!("{}/anime/{}", self.base_url, entry.route))
                    .send()?
                    .error_for_status()?;
                let html = response.text()?;
                let release = parse_release_info(&html);
                match (release.sub_episode, release.sub_countdown) {
                    (Some(episode), Some(countdown)) => {
                        println!("Next Sub: Episode {episode} in {countdown}");
                    }
                    _ => println!("Next Sub: N/A"),
                }
                match (release.dub_episode, release.dub_countdown) {
                    (Some(episode), Some(countdown)) => {
                        println!("Next Dub: Episode {episode} in {countdown}");
                    }
                    _ => println!("Next Dub: N/A"),
                }
            }
            println!();
        }
        Ok(())
    }
}

fn parse_entry(value: &Value) -> Option<AnimeScheduleEntry> {
    let route = value.get("route")?.as_str()?.trim();
    if route.is_empty() {
        return None;
    }
    let english_title = value
        .get("english")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .or_else(|| value.get("title").and_then(Value::as_str))
        .unwrap_or("N/A")
        .to_owned();
    let japanese_title = value
        .get("romaji")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("N/A")
        .to_owned();
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .filter(|status| !status.trim().is_empty())
        .unwrap_or("Unknown")
        .to_owned();

    Some(AnimeScheduleEntry {
        route: route.to_owned(),
        english_title,
        japanese_title,
        status,
    })
}

fn parse_release_info(html: &str) -> ReleaseInfo {
    let document = Html::parse_document(html);
    let sub_episode_selector =
        Selector::parse(".release-time-type-subs .release-time-episode-number")
            .expect("valid static CSS selector");
    let dub_episode_selector =
        Selector::parse(".release-time-type-dub .release-time-episode-number")
            .expect("valid static CSS selector");
    let countdown_selector =
        Selector::parse(".countdown-time[datetime]").expect("valid static CSS selector");

    let sub_episode = first_text(&document, &sub_episode_selector)
        .and_then(|text| text.strip_prefix("Episode").map(str::trim).map(str::to_owned));
    let dub_episode = first_text(&document, &dub_episode_selector)
        .and_then(|text| text.strip_prefix("Episode").map(str::trim).map(str::to_owned));

    let mut sub_countdown = None;
    let mut dub_countdown = None;
    for element in document.select(&countdown_selector) {
        let Some(datetime) = element.value().attr("datetime") else {
            continue;
        };
        let class = element.value().attr("class").unwrap_or_default();
        if class.split_whitespace().any(|name| name == "countdown-time-dub") {
            dub_countdown.get_or_insert_with(|| datetime.to_owned());
        } else {
            sub_countdown.get_or_insert_with(|| datetime.to_owned());
        }
    }

    ReleaseInfo {
        sub_episode,
        sub_countdown,
        dub_episode,
        dub_countdown,
    }
}

fn first_text(document: &Html, selector: &Selector) -> Option<String> {
    document
        .select(selector)
        .next()
        .map(|element| element.text().collect::<String>().trim().to_owned())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
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
}
