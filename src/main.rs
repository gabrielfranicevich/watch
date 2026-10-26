use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};

use ani_cli::cli::menu::{self, MenuOutcome};
use ani_cli::cli::selection::read_selection;
use ani_cli::domain::anime::Anime;
use ani_cli::domain::episode::Episode;
use ani_cli::domain::history::HistoryEntry;
use ani_cli::infrastructure::history_store::HistoryStore;
use ani_cli::infrastructure::paths;
use ani_cli::media::downloader::MediaDownloader;
use ani_cli::media::player::Player;
use ani_cli::providers::anime_schedule::AnimeScheduleProvider;
use ani_cli::providers::hianime::HiAnimeProvider;
use ani_cli::providers::hianime_stream::HiAnimeStreamResolver;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ani-cli: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = ani_cli::cli::args::Cli::parse(std::env::args().skip(1))?;
    if cli.help {
        print!("{}", ani_cli::cli::args::HELP);
        return Ok(ExitCode::SUCCESS);
    }
    if cli.version {
        println!("ani-cli {}", ani_cli::version());
        return Ok(ExitCode::SUCCESS);
    }

    let history = paths::history_file().map(HistoryStore::new);
    if cli.delete_history {
        let store = history
            .as_ref()
            .ok_or("cannot determine history path; set ANI_CLI_HIST_DIR or HOME")?;
        store.write(&[])?;
        println!("History deleted.");
        return Ok(ExitCode::SUCCESS);
    }
    if cli.logview {
        show_logs()?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.update {
        return Err("--update is not implemented yet: automatic self-update needs a verified Rust release artifact".into());
    }
    if cli.next_episode_countdown {
        let query = match cli.query.as_deref() {
            Some(query) => query.to_owned(),
            None => read_search_query()?,
        };
        AnimeScheduleProvider::new().show_countdown(&query)?;
        return Ok(ExitCode::SUCCESS);
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
        let selected_index = select_item(&labels, "Select anime: ", cli.select_index, &cli)?;
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
        let query = match cli.query.as_deref() {
            Some(query) => query.to_owned(),
            None => read_search_query()?,
        };
        eprintln!("Searching HiAnime for: {query}");
        let mut results = provider.search(&query)?;
        let exact = std::env::var("ANI_CLI_EXACT").is_ok_and(|value| value == "1");
        if exact {
            let normalized_query = normalize_title(query);
            results.retain(|result| normalize_title(&result.title) == normalized_query);
        }
        if results.is_empty() {
            let message = if exact {
                format!("no exact match found for '{query}'")
            } else {
                format!("no results found for '{query}'")
            };
            return Err(message.into());
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
        select_episode_range(episodes.clone(), range)?
    } else if let Some(last_watched) = resume_episode.as_deref() {
        let current = episodes
            .iter()
            .position(|episode| episode.number == last_watched)
            .ok_or_else(|| {
                format!("history episode {last_watched} is not present in the current episode list")
            })?;
        let next = episodes
            .get(current + 1)
            .ok_or("no next episode is available")?;
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
        let indices = select_items(&labels, "Select episode: ", None, true, &cli)?;
        indices
            .into_iter()
            .map(|index| episodes[index].clone())
            .collect()
    };

    let resolver = HiAnimeStreamResolver::new();
    let mode = std::env::var("ANI_CLI_MODE").unwrap_or_else(|_| "sub".to_owned());
    let dubbed = cli.dubbed || mode.eq_ignore_ascii_case("dub");
    let quality = cli
        .quality
        .clone()
        .or_else(|| std::env::var("ANI_CLI_QUALITY").ok())
        .unwrap_or_else(|| "best".to_owned());
    let env_player = std::env::var("ANI_CLI_PLAYER")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let player = Player::from_flags(
        cli.vlc || env_player.contains("vlc"),
        cli.syncplay || env_player.contains("syncplay"),
    );
    let player_flags: Vec<String> = std::env::var("ANI_CLI_PLAYER_FLAGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let download_dir = std::env::var_os("ANI_CLI_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if cli.download {
        std::fs::create_dir_all(&download_dir)?;
    }

    let mut playback = PlaybackConfig {
        resolver: &resolver,
        player,
        player_flags,
        dubbed,
        skip_intro: cli.skip_intro,
        quality,
        download: cli.download,
        download_dir: &download_dir,
        history: history.as_ref(),
    };
    let attached = cli.no_detach || cli.exit_after_play;
    let mut active_player: Option<Child> = None;

    for (index, episode) in selected.iter().enumerate() {
        if index > 0 {
            wait_for_player(&mut active_player)?;
        }
        match playback.play_episode(&anime, episode, attached)? {
            PlaybackResult::Detached(child) => active_player = Some(child),
            PlaybackResult::Attached(status) => {
                if cli.exit_after_play {
                    return Ok(exit_code(status));
                }
                ensure_success(status, player.command_name())?;
            }
            PlaybackResult::Downloaded => {}
        }
    }

    if cli.download {
        return Ok(ExitCode::SUCCESS);
    }

    let current_episode = selected.last().ok_or("no episode was selected")?;
    let mut current_index = episodes
        .iter()
        .position(|episode| episode.number == current_episode.number)
        .ok_or("selected episode is missing from the provider episode list")?;

    run_playback_menu(
        &anime,
        &episodes,
        &mut current_index,
        &mut active_player,
        &mut playback,
        &cli,
    )
}

struct PlaybackConfig<'a> {
    resolver: &'a HiAnimeStreamResolver,
    player: Player,
    player_flags: Vec<String>,
    dubbed: bool,
    skip_intro: bool,
    quality: String,
    download: bool,
    download_dir: &'a Path,
    history: Option<&'a HistoryStore>,
}

enum PlaybackResult {
    Detached(Child),
    Attached(ExitStatus),
    Downloaded,
}

impl PlaybackConfig<'_> {
    fn play_episode(
        &mut self,
        anime: &Anime,
        episode: &Episode,
        attached: bool,
    ) -> Result<PlaybackResult, Box<dyn std::error::Error>> {
        eprintln!(
            "Resolving episode {} ({}) at quality {}...",
            episode.number,
            if self.dubbed { "dub" } else { "sub" },
            self.quality
        );
        let media = self.resolver.resolve(episode, self.dubbed, &self.quality)?;
        let title = format!("{} - Episode {}", anime.title, episode.number);
        let mut media_flags = Vec::new();
        if self.skip_intro && matches!(self.player, Player::Mpv | Player::Syncplay) {
            let mal_id = media
                .mal_id
                .as_deref()
                .ok_or("--skip could not find a MAL ID in the embed URL")?;
            let output = Command::new("ani-skip")
                .args(["-i", mal_id, "-e", episode.number.as_str()])
                .output()?;
            ensure_success(output.status, "ani-skip")?;
            media_flags.extend(
                String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .map(str::to_owned),
            );
        }

        if self.download {
            let filename = format!(
                "{} - Episode {}.mp4",
                safe_filename(&anime.title),
                safe_filename(&episode.number)
            );
            let destination = self.download_dir.join(filename);
            let status = MediaDownloader::download(&media.stream, &destination)?;
            ensure_success(status, "ffmpeg")?;
            if let Some(subtitle) = media.subtitle.as_ref() {
                let subtitle_path = destination.with_extension("vtt");
                let mut request = reqwest::blocking::Client::new().get(&subtitle.url);
                if let Some(referer) = media.stream.referer.as_deref() {
                    request = request.header(reqwest::header::REFERER, referer);
                }
                let response = request.send()?.error_for_status()?;
                std::fs::write(subtitle_path, response.bytes()?)?;
            }
            self.record(anime, episode)?;
            return Ok(PlaybackResult::Downloaded);
        }

        if self.log_enabled() {
            let _ = Command::new("logger")
                .args([
                    "-t",
                    "ani-cli",
                    &format!("{} {}", anime.title, episode.number),
                ])
                .status();
        }

        if attached {
            let status = self.player.play_with_options(
                &media.stream,
                media.subtitle.as_ref(),
                &title,
                &self.player_flags,
                &media_flags,
            )?;
            self.record(anime, episode)?;
            Ok(PlaybackResult::Attached(status))
        } else {
            let child = self.player.launch_detached_with_options(
                &media.stream,
                media.subtitle.as_ref(),
                &title,
                &self.player_flags,
                &media_flags,
            )?;
            self.record(anime, episode)?;
            Ok(PlaybackResult::Detached(child))
        }
    }

    fn record(&self, anime: &Anime, episode: &Episode) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(store) = self.history {
            record_history(store, anime, episode)?;
        }
        Ok(())
    }

    fn log_enabled(&self) -> bool {
        std::env::var("ANI_CLI_LOG")
            .map(|value| value != "0")
            .unwrap_or(true)
    }
}

fn run_playback_menu(
    anime: &Anime,
    episodes: &[Episode],
    current_index: &mut usize,
    active_player: &mut Option<Child>,
    playback: &mut PlaybackConfig<'_>,
    cli: &ani_cli::cli::args::Cli,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let actions = vec![
        "Next episode".to_owned(),
        "Replay current episode".to_owned(),
        "Previous episode".to_owned(),
        "Select episode".to_owned(),
        "Change quality".to_owned(),
        "Quit".to_owned(),
    ];

    loop {
        let prompt = format!(
            "Playing episode {} of {} — choose action: ",
            episodes[*current_index].number, anime.title
        );
        let action = match select_item(&actions, &prompt, None, cli) {
            Ok(index) => index,
            Err(error) if error.to_string() == "selection cancelled" => {
                return Ok(ExitCode::SUCCESS);
            }
            Err(error) => return Err(error),
        };

        match action {
            0 => {
                if *current_index + 1 >= episodes.len() {
                    eprintln!("Already at the last episode.");
                    continue;
                }
                *current_index += 1;
            }
            1 => {}
            2 => {
                if *current_index == 0 {
                    eprintln!("Already at the first episode.");
                    continue;
                }
                *current_index -= 1;
            }
            3 => {
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
                *current_index = select_item(&labels, "Select episode: ", None, cli)?;
            }
            4 => {
                print!("Video quality (best, worst, 1080p, 720p, ...): ");
                io::stdout().flush()?;
                let mut value = String::new();
                if io::stdin().read_line(&mut value)? == 0 {
                    return Ok(ExitCode::SUCCESS);
                }
                let value = value.trim();
                if value.is_empty() {
                    eprintln!("Quality was not changed.");
                    continue;
                }
                playback.quality = value.to_owned();
            }
            _ => return Ok(ExitCode::SUCCESS),
        }

        wait_for_player(active_player)?;
        match playback.play_episode(anime, &episodes[*current_index], cli.no_detach)? {
            PlaybackResult::Detached(child) => *active_player = Some(child),
            PlaybackResult::Attached(status) => {
                ensure_success(status, playback.player.command_name())?
            }
            PlaybackResult::Downloaded => return Ok(ExitCode::SUCCESS),
        }
    }
}

fn wait_for_player(active_player: &mut Option<Child>) -> io::Result<()> {
    if let Some(mut child) = active_player.take() {
        let _ = child.wait()?;
    }
    Ok(())
}

fn exit_code(status: ExitStatus) -> ExitCode {
    ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8)
}

fn ensure_success(status: ExitStatus, program: &str) -> Result<(), Box<dyn std::error::Error>> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}").into())
    }
}

fn show_logs() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let status = Command::new("log")
        .args(["show", "--predicate", "process == \"logger\""])
        .status()?;

    #[cfg(target_os = "linux")]
    let status = Command::new("journalctl")
        .args(["-t", "ani-cli"])
        .status()?;

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("--logview is supported only on Linux and macOS".into());

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    ensure_success(status, "log viewer")
}

fn record_history(store: &HistoryStore, anime: &Anime, episode: &Episode) -> io::Result<()> {
    let mut entries = store.read()?;
    entries.retain(|entry| entry.anime_id != anime.id);
    entries.push(HistoryEntry::new(
        episode.number.clone(),
        anime.id.clone(),
        anime.title.clone(),
    ));
    store.write(&entries)
}

fn normalize_title(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn select_episode_range(
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
                let start_number: f64 = start.parse().map_err(|_| {
                    format!("invalid start episode '{start}' in selection '{token}'")
                })?;
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
                    return Err(format!(
                        "episode range '{token}' did not match available episodes"
                    )
                    .into());
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

fn select_item(
    labels: &[String],
    prompt: &str,
    forced_one_based: Option<usize>,
    cli: &ani_cli::cli::args::Cli,
) -> Result<usize, Box<dyn std::error::Error>> {
    select_items(labels, prompt, forced_one_based, false, cli)?
        .into_iter()
        .next()
        .ok_or_else(|| "menu returned no selection".into())
}

fn select_items(
    labels: &[String],
    prompt: &str,
    forced_one_based: Option<usize>,
    multi: bool,
    cli: &ani_cli::cli::args::Cli,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
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
        return Ok(vec![index]);
    }

    let backend = if cli.rofi {
        Some("rofi".to_owned())
    } else if cli.dmenu {
        Some("dmenu".to_owned())
    } else {
        std::env::var("ANI_CLI_MENU").ok()
    };

    match menu::select(labels, prompt, backend.as_deref(), multi)? {
        MenuOutcome::Selected(mut indices) => {
            indices.sort_unstable();
            indices.dedup();
            if indices.is_empty() {
                Err("selection cancelled".into())
            } else {
                Ok(indices)
            }
        }
        MenuOutcome::Cancelled => Err("selection cancelled".into()),
        MenuOutcome::Unavailable => {
            for (index, label) in labels.iter().enumerate() {
                println!("{:>2}. {}", index + 1, label);
            }
            Ok(vec![read_index(labels.len())?])
        }
    }
}

fn read_search_query() -> Result<String, Box<dyn std::error::Error>> {
    print!("Search anime: ");
    io::stdout().flush()?;
    let mut query = String::new();
    io::stdin().read_line(&mut query)?;
    let query = query.trim();
    if query.is_empty() {
        return Err("search query was empty or cancelled".into());
    }
    Ok(query.to_owned())
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_replaces_platform_invalid_characters() {
        assert_eq!(safe_filename("A/B: C?"), "A_B_ C_");
    }

    #[test]
    fn supports_multiple_episode_numbers_and_preserves_provider_order() {
        let episodes = vec![
            Episode::new("3", None, "three"),
            Episode::new("2", None, "two"),
            Episode::new("1", None, "one"),
        ];
        let selected = select_episode_range(episodes, "1 3").unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|episode| episode.number.as_str())
                .collect::<Vec<_>>(),
            vec!["3", "1"]
        );
    }

    #[test]
    fn supports_first_last_and_numeric_episode_ranges() {
        let episodes = vec![
            Episode::new("1", None, "one"),
            Episode::new("1.5", None, "one and a half"),
            Episode::new("2", None, "two"),
            Episode::new("3", None, "three"),
        ];
        assert_eq!(
            select_episode_range(episodes.clone(), "0 -1")
                .unwrap()
                .iter()
                .map(|ep| ep.number.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "3"]
        );
        assert_eq!(
            select_episode_range(episodes, "1-2")
                .unwrap()
                .iter()
                .map(|ep| ep.number.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "1.5", "2"]
        );
    }

    #[test]
    fn normalizes_titles_for_exact_match() {
        assert_eq!(normalize_title("  BLUE   lock\n"), "blue lock");
    }

    #[test]
    fn selects_multiple_episode_numbers_in_provider_order() {
        let episodes = vec![
            Episode::new("3", None, "three"),
            Episode::new("2", None, "two"),
            Episode::new("1", None, "one"),
        ];
        let selected = select_episode_range(episodes, "1 3").unwrap();
        assert_eq!(
            selected.iter().map(|episode| episode.number.as_str()).collect::<Vec<_>>(),
            vec!["3", "1"]
        );
    }
}
