//! Application-level policies shared by the CLI orchestration and regression tests.

use crate::domain::episode::Episode;

pub fn effective_source(
  source_override: Option<crate::cli::args::SourceOverride>,
  configured: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
  let source = match source_override {
    Some(crate::cli::args::SourceOverride::History) => "history",
    Some(crate::cli::args::SourceOverride::NextEpisode) => "nextep",
    None => configured.unwrap_or("search"),
  };

  match source {
    "search" | "history" | "nextep" => Ok(source.to_owned()),
    other => Err(
      format!("invalid ANI_CLI_DEFAULT_SOURCE '{other}'; expected search, history, or nextep")
        .into(),
    ),
  }
}

pub fn select_episode_range(
  episodes: Vec<Episode>,
  range: &str,
) -> Result<Vec<Episode>, Box<dyn std::error::Error>> {
  if episodes.is_empty() {
    return Err("cannot select episodes from an empty list".into());
  }
  let available_numbers: Vec<&str> = episodes
    .iter()
    .map(|episode| episode.number.as_str())
    .collect();
  let mut requested = Vec::<String>::new();

  for token in range.split_whitespace() {
    match token {
      "0" => requested.push(available_numbers[0].to_owned()),
      "-1" => requested.push(available_numbers[available_numbers.len() - 1].to_owned()),
      _ if token.contains('-') => {
        let (start, end) = token
          .split_once('-')
          .ok_or_else(|| format!("invalid episode selection '{token}'"))?;
        let start = if start == "0" {
          available_numbers[0]
        } else {
          start
        };
        let end = if end == "-1" {
          available_numbers[available_numbers.len() - 1]
        } else {
          end
        };
        let start_number: f64 = start
          .parse()
          .map_err(|_| format!("invalid start episode '{start}' in selection '{token}'"))?;
        let end_number: f64 = end
          .parse()
          .map_err(|_| format!("invalid end episode '{end}' in selection '{token}'"))?;
        if end_number < start_number {
          return Err(format!("invalid episode range '{token}'").into());
        }
        let matched: Vec<String> = available_numbers
          .iter()
          .filter_map(|number| {
            number.parse::<f64>().ok().and_then(|number_value| {
              (number_value >= start_number && number_value <= end_number)
                .then(|| (*number).to_owned())
            })
          })
          .collect();
        if matched.is_empty() {
          return Err(format!("episode range '{token}' did not match available episodes").into());
        }
        requested.extend(matched);
      }
      _ => {
        if !available_numbers.contains(&token) {
          return Err(format!("episode '{token}' is not available").into());
        }
        requested.push(token.to_owned());
      }
    }
  }

  if requested.is_empty() {
    return Err("no episode numbers were selected".into());
  }
  let selected: Vec<Episode> = episodes
    .into_iter()
    .filter(|episode| requested.contains(&episode.number))
    .collect();
  if selected.is_empty() {
    return Err(format!("episode range '{range}' did not match available episodes").into());
  }
  Ok(selected)
}

pub fn select_fixed_episodes(
  episodes: &[Episode],
  range: Option<&str>,
  resume_episode: Option<&str>,
) -> Result<Option<Vec<Episode>>, Box<dyn std::error::Error>> {
  // The shell implementation's history source overrides an explicit -e value.
  if let Some(last_watched) = resume_episode {
    let next = next_episode_after(episodes, last_watched)
      .ok_or("no next episode is available after the last watched episode")?;
    return Ok(Some(vec![next.clone()]));
  }
  if let Some(range) = range {
    return Ok(Some(select_episode_range(episodes.to_vec(), range)?));
  }
  Ok(None)
}

pub fn normalize_title(title: &str) -> String {
  title
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase()
}

pub fn next_episode_after<'a>(episodes: &'a [Episode], last_watched: &str) -> Option<&'a Episode> {
  let index = episodes
    .iter()
    .position(|episode| episode.number == last_watched)?;
  episodes.get(index + 1)
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
  Automatic,
  Inline,
  External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackTarget {
  Inline,
  External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackPlatform {
  Desktop,
  Android,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackCapabilities {
  pub interactive_terminal: bool,
  pub terminal_graphics_supported: bool,
}

pub fn choose_playback_target(
  mode: PlaybackMode,
  platform: PlaybackPlatform,
  capabilities: PlaybackCapabilities,
) -> Result<PlaybackTarget, String> {
  let inline_supported = platform == PlaybackPlatform::Desktop
    && capabilities.interactive_terminal
    && capabilities.terminal_graphics_supported;

  match mode {
    PlaybackMode::External => Ok(PlaybackTarget::External),
    PlaybackMode::Automatic if inline_supported => Ok(PlaybackTarget::Inline),
    PlaybackMode::Automatic => Ok(PlaybackTarget::External),
    PlaybackMode::Inline if inline_supported => Ok(PlaybackTarget::Inline),
    PlaybackMode::Inline if platform == PlaybackPlatform::Android => Err(
      "inline playback is not supported in Termux; omit --inline/--here to use external playback"
        .to_owned(),
    ),
    PlaybackMode::Inline if !capabilities.interactive_terminal => Err(
      "--inline requires an interactive terminal on stdin and stdout".to_owned(),
    ),
    PlaybackMode::Inline => Err(
      "inline playback requires a terminal with a supported graphics protocol".to_owned(),
    ),
  }
}

#[cfg(test)]
#[path = "../tests/unit/playback_policy.rs"]
mod playback_policy_tests;
