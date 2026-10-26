use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ani_cli::cli::menu::{self, MenuOutcome};
use ani_cli::cli::selection::read_selection;
use ani_cli::domain::anime::Anime;
use ani_cli::domain::episode::Episode;
use ani_cli::domain::history::HistoryEntry;
use ani_cli::infrastructure::history_store::HistoryStore;
use ani_cli::infrastructure::paths;
use ani_cli::media::downloader::FfmpegDownloader;
use ani_cli::media::player::Player;
use ani_cli::providers::hianime::HiAnimeProvider;
use ani_cli::providers::hianime_stream::HiAnimeStreamResolver;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ani-cli: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = ani_cli::cli::args::Cli::parse(std::env::args().skip(1))?;
    if cli.help {
        print!("{}", ani_cli::cli::args::HELP);
        return Ok(());
    }
    if cli.version {
        println!("ani-cli {}", ani_cli::version());
        return Ok(());
    }

    let history = paths::history_file().map(HistoryStore::new);
    if cli.delete_history {
        let store = history
            .as_ref()
            .ok_or("cannot determine history path; set ANI_CLI_HIST_DIR or HOME")?;
        store.write(&[])?;
        println!("History deleted.");
        return Ok(());
    }
    if cli.logview {
        return show_logs();
    }
    if cli.update {
        return Err("--update is not implemented yet: automatic self-update needs a verified Rust release artifact".into());
    }
    if cli.next_episode_countdown {
        return Err("--nextep-countdown is not implemented in the Rust migration yet".into());
    }
    if cli.syncplay || cli.skip_intro {
        return Err("--syncplay and --skip are not implemented in the Rust migration yet".into());
    }

    let provider = HiAnimeProvider::new();
    let (anime, resume_episode) = if cli.continue_history {
        let store = history
            .as_ref()
            .ok_or("cannot determine history path; set ANI_CLI_HIST_DIR or HOME")?;
        let entries = store.read()?;
        if entries.is_empty() {
            return Err("watch history is empty".into());
        }

        let mut recent = Vec::<HistoryEntry>::new();
        for entry in entries.into_iter().rev() {
            if !recent.iter().any(|item| item.anime_id == entry.anime_id) {
                recent.push(entry);
            }
        }
        let labels: Vec<String> = recent
            .iter()
            .map(|entry| {
                format!(
                    "{} — last watched episode {}",
                    entry.anime_title, entry.episode_number
                )
            })
            .collect();
        let selected_index =
            select_item(&labels, "Select anime: ", cli.select_index, &cli)?;
        let entry = &recent[selected_index];
        (
            Anime::new(
                entry.anime_id.clone(),
                entry.anime_title.clone(),
                format!("https://hianime.at/{}", entry.anime_id),
            ),
            Some(entry.episode_number.clone()),
        )
    } else {
        let query = cli.query.as_deref().ok_or("a search query is required")?;
        eprintln!("Searching HiAnime for: {query}");
        let results = provider.search(query)?;
        if results.is_empty() {
            return Err(format!("no results found for '{query}'").into());
        }
        let labels: Vec<String> = results.iter().map(|result| result.title.clone()).collect();
        let index = select_item(&labels, "Select anime: ", cli.select_index, &cli)?;
        (results[index].clone(), None)
    };

    println!("{}\n{}", anime.title, anime.url);
    let episodes = provider.episodes(&anime)?;
    if episodes.is_empty() {
        return Err(format!("no episodes found for '{}'", anime.title).into());
    }

    let selected: Vec<Episode> = if let Some(range) = cli.episode_range.as_deref() {
        select_episode_range(episodes, range)?
    } else if let Some(last_watched) = resume_episode.as_deref() {
        let current = episodes
            .iter()
            .position(|episode| episode.number == last_watched)
            .ok_or_else(|| {
                format!(
                    "history episode {last_watched} is not present in the current episode list"
                )
            })?;
        let next = episodes.get(current + 1).ok_or("no next episode is available")?;
        vec![next.clone()]
    } else {
        let labels: Vec<String> = episodes
            .iter()
            .map(|episode| {
                format!(
                    "Episode {}{}",
                    episode.number,
                    episode
                        .title
                        .as_deref()
                        .map(|title| format!(" — {title}"))
                        .unwrap_or_default()
                )
            })
            .collect();
        let index = select_item(&labels, "Select episode: ", None, &cli)?;
        vec![episodes[index].clone()]
    };

    let resolver = HiAnimeStreamResolver::new();
    let mode = std::env::var("ANI_CLI_MODE").unwrap_or_else(|_| "sub".to_owned());
    let dubbed = cli.dubbed || mode.eq_ignore_ascii_case("dub");
    let quality = cli
        .quality
        .clone()
        .or_else(|| std::env::var("ANI_CLI_QUALITY").ok())
        .unwrap_or_else(|| "best".to_owned());
    let player = Player::from_vlc_flag(
        cli.vlc
            || std::env::var("ANI_CLI_PLAYER")
                .is_ok_and(|value| value.to_ascii_lowercase().contains("vlc")),
    );
    let download_dir = std::env::var_os("ANI_CLI_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if cli.download {
        std::fs::create_dir_all(&download_dir)?;
    }

    for episode in &selected {
        eprintln!(
            "Resolving episode {} ({})...",
            episode.number,
            if dubbed { "dub" } else { "sub" }
        );
        let media = resolver.resolve(episode, dubbed, &quality)?;
        let title = format!("{} - Episode {}", anime.title, episode.number);

        if cli.download {
            let filename = format!(
                "{} - Episode {}.mp4",
                safe_filename(&anime.title),
                safe_filename(&episode.number)
            );
            let destination = download_dir.join(filename);
            let status = FfmpegDownloader::download(&media.stream, &destination)?;
            if !status.success() {
                return Err(format!("ffmpeg exited with status {status}").into());
            }
            if let Some(subtitle) = media.subtitle.as_ref() {
                let subtitle_path = destination.with_extension("vtt");
                let mut request = reqwest::blocking::Client::new().get(&subtitle.url);
                if let Some(referer) = media.stream.referer.as_deref() {
                    request = request.header(reqwest::header::REFERER, referer);
                }
                let response = request.send()?.error_for_status()?;
                std::fs::write(subtitle_path, response.bytes()?)?;
            }
        } else {
            let status = player.play(&media.stream, media.subtitle.as_ref(), &title)?;
            if !status.success() {
                return Err(
                    format!("{} exited with status {status}", player.command_name()).into(),
                );
            }
        }

        if let Some(store) = history.as_ref() {
            record_history(store, &anime, episode)?;
        }
    }

    Ok(())
}

fn show_logs() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("log")
        .args(["show", "--predicate", "process == \"logger\""])
        .status()?;

    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("journalctl")
        .args(["-t", "ani-cli"])
        .status()?;

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("--logview is supported only on Linux and macOS".into());

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if !status.success() {
        return Err(format!("log viewer exited with status {status}").into());
    }

    Ok(())
}

fn record_history(
    store: &HistoryStore,
    anime: &Anime,
    episode: &Episode,
) -> io::Result<()> {
    let mut entries = store.read()?;
    if let Some(existing) = entries.iter_mut().find(|entry| entry.anime_id == anime.id) {
        existing.episode_number = episode.number.clone();
        existing.anime_title = anime.title.clone();
    } else {
        entries.push(HistoryEntry::new(
            episode.number.clone(),
            anime.id.clone(),
            anime.title.clone(),
        ));
    }
    store.write(&entries)
}

fn select_episode_range(
    episodes: Vec<Episode>,
    range: &str,
) -> Result<Vec<Episode>, Box<dyn std::error::Error>> {
    let requested = parse_episode_range(range)?;
    let selected: Vec<_> = episodes
        .into_iter()
        .filter(|episode| requested.iter().any(|number| number == &episode.number))
        .collect();
    if selected.is_empty() {
        return Err(format!("episode range '{range}' did not match available episodes").into());
    }
    Ok(selected)
}

fn select_item(
    labels: &[String],
    prompt: &str,
    forced_one_based: Option<usize>,
    cli: &ani_cli::cli::args::Cli,
) -> Result<usize, Box<dyn std::error::Error>> {
    if let Some(number) = forced_one_based {
        let index = number
            .checked_sub(1)
            .ok_or("-S expects a positive selection number")?;
        if index >= labels.len() {
            return Err(format!(
                "selection {number} is outside the valid range 1-{}",
                labels.len()
            )
            .into());
        }
        return Ok(index);
    }

    let backend = if cli.rofi {
        Some("rofi".to_owned())
    } else if cli.dmenu {
        Some("dmenu".to_owned())
    } else {
        std::env::var("ANI_CLI_MENU").ok()
    };

    match menu::select(labels, prompt, backend.as_deref(), false)? {
        MenuOutcome::Selected(indices) => indices
            .first()
            .copied()
            .ok_or_else(|| "menu returned no selection".into()),
        MenuOutcome::Cancelled => Err("selection cancelled".into()),
        MenuOutcome::Unavailable => {
            for (index, label) in labels.iter().enumerate() {
                println!("{:>2}. {}", index + 1, label);
            }
            read_index(labels.len())
        }
    }
}

fn read_index(count: usize) -> Result<usize, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    read_selection(&mut input, &mut output, count)?.ok_or_else(|| "selection cancelled".into())
}

fn safe_filename(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control()
                || ['<', '>', ':', '"', '/', '\\', '|', '?', '*'].contains(&character)
            {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn parse_episode_range(range: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if let Some((start, end)) = range.split_once('-') {
        let start: u32 = start.parse()?;
        let end: u32 = end.parse()?;
        if start == 0 || end < start || end - start > 1000 {
            return Err(format!("invalid episode range '{range}'").into());
        }
        Ok((start..=end).map(|number| number.to_string()).collect())
    } else {
        let number: u32 = range.parse()?;
        if number == 0 {
            return Err("episode numbers start at 1".into());
        }
        Ok(vec![number.to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_replaces_platform_invalid_characters() {
        assert_eq!(safe_filename("A/B: C?"), "A_B_ C_");
    }

    #[test]
    fn parses_single_episode_and_ranges() {
        assert_eq!(parse_episode_range("4").unwrap(), vec!["4"]);
        assert_eq!(parse_episode_range("2-4").unwrap(), vec!["2", "3", "4"]);
        assert!(parse_episode_range("0").is_err());
        assert!(parse_episode_range("4-2").is_err());
    }

    #[test]
    fn filters_requested_episodes_in_provider_order() {
        let episodes = vec![
            Episode::new("3", None, "three"),
            Episode::new("2", None, "two"),
            Episode::new("1", None, "one"),
        ];
        let selected = select_episode_range(episodes, "1-2").unwrap();
        assert_eq!(
            selected.iter().map(|episode| episode.number.as_str()).collect::<Vec<_>>(),
            vec!["2", "1"]
        );
    }
}
