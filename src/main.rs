use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};

use ani_cli::application::{effective_source, next_episode_after, normalize_title, select_fixed_episodes};
#[cfg(test)]
use ani_cli::application::select_episode_range;
use ani_cli::cli::menu::{self, MenuOutcome, TextPromptOutcome};
use ani_cli::cli::selection::read_selection;
use ani_cli::domain::anime::Anime;
use ani_cli::domain::episode::Episode;
use ani_cli::domain::history::HistoryEntry;
use ani_cli::infrastructure::history_store::HistoryStore;
use ani_cli::infrastructure::paths;
use ani_cli::media::downloader::{MediaDownloader, SubtitleDownloader};
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
        let branch = cli
            .query
            .clone()
            .or_else(|| std::env::var("ANI_CLI_BRANCH").ok())
            .unwrap_or_else(|| "develop".to_owned());
        update_rust_installation(&branch)?;
        return Ok(ExitCode::SUCCESS);
    }
    let source = effective_source(
        cli.source_override,
        std::env::var("ANI_CLI_DEFAULT_SOURCE").ok().as_deref(),
    )?;
    let mut search_query = cli.query.clone();
    if source == "nextep" {
        let query = match search_query.clone() {
            Some(query) => query,
            None => read_search_query(&cli)?,
        };
        if let Err(error) = AnimeScheduleProvider::new().show_countdown(&query) {
            eprintln!("ani-cli: warning: could not fetch next-episode countdown: {error}");
        }
        // The original script prints the countdown and then continues into
        // the normal anime search/playback flow using the same query.
        search_query = Some(query);
    }

    let provider = HiAnimeProvider::new();
    let (anime, resume_episode, prefetched_episodes) = if source == "history" {
        let store = history
            .as_ref()
            .ok_or("cannot determine history path; set ANI_CLI_HIST_DIR or HOME")?;
        store.backup_legacy_entries()?;
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

        let mut candidates = Vec::new();
        for entry in recent {
            let candidate_anime = Anime::new(
                entry.anime_id.clone(),
                entry.anime_title.clone(),
                format!("https://hianime.at/{}", entry.anime_id),
            );
            let candidate_episodes = match provider.episodes(&candidate_anime) {
                Ok(episodes) => episodes,
                Err(error) => {
                    eprintln!(
                        "Skipping '{}' in history because its episode list could not be loaded: {error}",
                        entry.anime_title
                    );
                    continue;
                }
            };
            if !candidate_episodes
                .iter()
                .any(|episode| episode.number == entry.episode_number)
            {
                eprintln!(
                    "Skipping '{}' in history because episode {} is no longer available",
                    entry.anime_title, entry.episode_number
                );
                continue;
            }
            let Some(next_episode) = next_episode_after(&candidate_episodes, &entry.episode_number)
            else {
                eprintln!(
                    "Skipping '{}' in history because it has no next episode",
                    entry.anime_title
                );
                continue;
            };
            let next_episode_number = next_episode.number.clone();
            candidates.push((
                entry,
                candidate_anime,
                candidate_episodes,
                next_episode_number,
            ));
        }
        if candidates.is_empty() {
            return Err("no unwatched series in history".into());
        }

        let labels: Vec<String> = candidates
            .iter()
            .map(|(entry, _, _, next_episode)| {
                format!("{} — next episode {}", entry.anime_title, next_episode)
            })
            .collect();
        let selected_index = select_item(&labels, "Select anime: ", cli.select_index, &cli)?;
        let (entry, candidate_anime, episodes, _) = candidates.remove(selected_index);
        (candidate_anime, Some(entry.episode_number), Some(episodes))
    } else {
        let query = match search_query.as_deref() {
            Some(query) => query.to_owned(),
            None => read_search_query(&cli)?,
        };
        eprintln!("Searching HiAnime for: {query}");
        let mut results = provider.search(&query)?;
        let exact = std::env::var("ANI_CLI_EXACT").is_ok_and(|value| value == "1");
        if exact {
            let normalized_query = normalize_title(&query);
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
        (results[index].clone(), None, None)
    };

    println!("{}\n{}", anime.title, anime.url);
    let episodes = match prefetched_episodes {
        Some(episodes) => episodes,
        None => provider.episodes(&anime)?,
    };
    if episodes.is_empty() {
        return Err(format!("no episodes found for '{}'", anime.title).into());
    }

    let selected: Vec<Episode> = if let Some(selected) = select_fixed_episodes(
        &episodes,
        cli.episode_range.as_deref(),
        resume_episode.as_deref(),
    )? {
        selected
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
    let env_player = std::env::var("ANI_CLI_PLAYER").ok();
    let default_iina_path = "/Applications/IINA.app/Contents/MacOS/iina-cli";
    let use_iina_default = env_player.is_none()
        && cfg!(target_os = "macos")
        && (command_available("iina") || Path::new(default_iina_path).is_file());
    let use_flatpak_default = env_player.is_none()
        && !cli.vlc
        && !cli.syncplay
        && cfg!(target_os = "linux")
        && !command_available("mpv")
        && command_available("flatpak")
        && flatpak_mpv_installed();
    let player = if !player_required(cli.download) {
        // Download-only mode does not spawn a player and must work without MPV/VLC.
        Player::Mpv
    } else if cli.syncplay || cli.vlc || env_player.is_some() || cfg!(target_os = "android") {
        Player::from_configuration(cli.vlc, cli.syncplay, false, env_player.as_deref())
    } else if use_iina_default && command_available("iina") {
        Player::from_configuration(false, false, false, Some("iina"))
    } else if use_iina_default {
        Player::Iina
    } else if use_flatpak_default {
        Player::FlatpakMpv
    } else if cfg!(target_os = "windows") || command_available("mpv") {
        Player::Mpv
    } else if command_available("vlc") {
        Player::Vlc
    } else {
        return Err("no supported player found; install mpv or VLC, or set ANI_CLI_PLAYER".into());
    };
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

    let exit_after_play = cli.exit_after_play || env_enabled("ANI_CLI_EXIT_AFTER_PLAY");
    let attached = cli.no_detach || env_enabled("ANI_CLI_NO_DETACH") || exit_after_play;
    let mut playback = PlaybackConfig {
        resolver: &resolver,
        player,
        player_flags,
        dubbed,
        skip_intro: cli.skip_intro || env_enabled("ANI_CLI_SKIP_INTRO"),
        quality,
        available_qualities: Vec::new(),
        download: cli.download,
        download_dir: &download_dir,
        history: history.as_ref(),
    };
    let mut active_player: Option<Child> = None;

    for (index, episode) in selected.iter().enumerate() {
        if index > 0 {
            wait_for_player(&mut active_player)?;
        }
        match playback.play_episode(&anime, episode, attached)? {
            PlaybackResult::Detached(child) => active_player = Some(child),
            PlaybackResult::Attached(status) => {
                if exit_after_play && index + 1 == selected.len() {
                    return Ok(exit_code(status));
                }
                ensure_success(status, playback.player.command_name())?;
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
        attached,
    )
}

struct PlaybackConfig<'a> {
    resolver: &'a HiAnimeStreamResolver,
    player: Player,
    player_flags: Vec<String>,
    dubbed: bool,
    skip_intro: bool,
    quality: String,
    available_qualities: Vec<String>,
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
        self.available_qualities = media
            .qualities
            .iter()
            .map(|option| option.value.clone())
            .collect();
        let title = format!("{} - Episode {}", anime.title, episode.number);
        let mut media_flags = Vec::new();
        if self.skip_intro && self.player.supports_intro_skipping() {
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
            let filename = format!("{}.mp4", episode_file_stem(&anime.title, &episode.number));
            let destination = self.download_dir.join(filename);
            if let Some(subtitle) = media.subtitle.as_ref() {
                let subtitle_path = destination.with_extension("vtt");
                if let Err(error) = SubtitleDownloader::download(
                    &subtitle.url,
                    media.stream.referer.as_deref(),
                    &subtitle_path,
                ) {
                    eprintln!("ani-cli: warning: could not download subtitles: {error}");
                }
            }

            let status = MediaDownloader::download_with_extra(
                &media.stream,
                &destination,
                &self.player_flags,
            )?;
            ensure_success(status, "ffmpeg")?;
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
    attached: bool,
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

        let mut episodes_to_play = Vec::new();
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
                episodes_to_play = select_items(&labels, "Select episode: ", None, true, cli)?;
            }
            4 => {
                let choices = quality_choices(&playback.available_qualities);
                let labels: Vec<String> = choices
                    .iter()
                    .map(|(label, _)| {
                        if label == &playback.quality {
                            format!("{label} (current)")
                        } else {
                            label.clone()
                        }
                    })
                    .collect();
                let selected = match select_item(&labels, "Select quality: ", None, cli) {
                    Ok(index) => index,
                    Err(error) if error.to_string() == "selection cancelled" => continue,
                    Err(error) => return Err(error),
                };
                playback.quality = choices[selected].1.clone();
            }
            _ => return Ok(ExitCode::SUCCESS),
        }

        if episodes_to_play.is_empty() {
            episodes_to_play.push(*current_index);
        }

        wait_for_player(active_player)?;
        for (position, episode_index) in episodes_to_play.into_iter().enumerate() {
            if position > 0 {
                wait_for_player(active_player)?;
            }
            *current_index = episode_index;
            match playback.play_episode(anime, &episodes[*current_index], attached)? {
                PlaybackResult::Detached(child) => *active_player = Some(child),
                PlaybackResult::Attached(status) => {
                    ensure_success(status, playback.player.command_name())?
                }
                PlaybackResult::Downloaded => return Ok(ExitCode::SUCCESS),
            }
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

fn update_rust_installation(branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !valid_branch_name(branch) {
        return Err(format!("invalid Git branch name '{branch}'").into());
    }

    let executable = std::env::current_exe()?;
    let bin_dir = executable
        .parent()
        .ok_or("cannot determine the current executable directory")?;
    if bin_dir.file_name() != Some(OsStr::new("bin")) {
        return Err(format!(
            "cannot infer Cargo installation root from {}; update manually or install with cargo install",
            executable.display()
        )
        .into());
    }
    let install_root = bin_dir
        .parent()
        .ok_or("cannot determine Cargo installation root")?;

    eprintln!("Updating ani-cli from branch '{branch}'...");
    let status = Command::new("cargo")
        .args(updater_arguments(install_root, branch))
        .status()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "cargo was not found; install the Rust toolchain to use --update",
                )
            } else {
                error
            }
        })?;
    ensure_success(status, "cargo install")?;
    eprintln!("ani-cli update completed.");
    Ok(())
}

fn updater_arguments(install_root: &Path, branch: &str) -> Vec<OsString> {
    vec![
        OsString::from("install"),
        OsString::from("--force"),
        OsString::from("--git"),
        OsString::from("https://github.com/gabrielfranicevich/anime"),
        OsString::from("--branch"),
        OsString::from(branch),
        OsString::from("--root"),
        install_root.as_os_str().to_owned(),
    ]
}

fn valid_branch_name(branch: &str) -> bool {
    !branch.is_empty()
        && !branch.starts_with('-')
        && !branch.contains("..")
        && branch
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_./".contains(character))
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

fn quality_choices(available: &[String]) -> Vec<(String, String)> {
    let mut choices = vec![
        ("best".to_owned(), "best".to_owned()),
        ("worst".to_owned(), "worst".to_owned()),
    ];
    let mut options = available.to_vec();
    options.sort_by(|left, right| {
        quality_height(right)
            .cmp(&quality_height(left))
            .then_with(|| left.cmp(right))
    });
    options.dedup();
    for option in options {
        if !choices.iter().any(|(_, value)| value == &option) {
            choices.push((option.clone(), option));
        }
    }
    choices
}

fn quality_height(value: &str) -> u32 {
    value
        .trim_end_matches('p')
        .parse::<u32>()
        .unwrap_or_default()
}

fn ordered_episode_indices(mut indices: Vec<usize>) -> Vec<usize> {
    indices.sort_unstable();
    indices.dedup();
    indices
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

    let backend = menu_backend_for_cli(cli);

    match menu::select(labels, prompt, backend.as_deref(), multi)? {
        MenuOutcome::Selected(indices) => {
            let indices = ordered_episode_indices(indices);
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

fn menu_backend_for_cli(cli: &ani_cli::cli::args::Cli) -> Option<String> {
    let stdin_is_terminal = io::stdin().is_terminal();
    let configured_backend = std::env::var("ANI_CLI_MENU").ok();
    preferred_menu_backend(
        cli,
        stdin_is_terminal,
        configured_backend.as_deref(),
        !stdin_is_terminal && command_available("dmenu"),
        !stdin_is_terminal && command_available("rofi"),
    )
}

fn preferred_menu_backend(
    cli: &ani_cli::cli::args::Cli,
    stdin_is_terminal: bool,
    configured: Option<&str>,
    dmenu_available: bool,
    rofi_available: bool,
) -> Option<String> {
    if cli.rofi {
        return Some("rofi".to_owned());
    }
    if cli.dmenu {
        return Some("dmenu".to_owned());
    }

    // Match the shell implementation: on non-interactive stdin, prefer dmenu
    // and let rofi override it when both are installed.
    if !stdin_is_terminal {
        if dmenu_available {
            let default = Some("dmenu".to_owned());
            if rofi_available {
                return Some("rofi".to_owned());
            }
            return default;
        }
        if rofi_available {
            return Some("rofi".to_owned());
        }
    }

    configured
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn player_required(download_only: bool) -> bool {
    !download_only
}

fn flatpak_mpv_installed() -> bool {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs_home);
    home.is_some_and(|home| home.join(".local/share/flatpak/app/io.mpv.Mpv").is_dir())
        || Path::new("/var/lib/flatpak/app/io.mpv.Mpv").is_dir()
}

fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn command_available(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|directory| {
        if directory.join(program).is_file() {
            return true;
        }

        #[cfg(windows)]
        {
            let extensions = std::env::var_os("PATHEXT")
                .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
            extensions
                .to_string_lossy()
                .split(';')
                .filter(|extension| !extension.is_empty())
                .any(|extension| directory.join(format!("{program}{extension}")).is_file())
        }

        #[cfg(not(windows))]
        {
            false
        }
    })
}

fn env_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| is_enabled(Some(&value)))
}

fn is_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}



fn read_search_query(cli: &ani_cli::cli::args::Cli) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(backend) = menu_backend_for_cli(cli) {
        match menu::prompt_text("Search anime: ", Some(&backend))? {
            TextPromptOutcome::Text(query) => return Ok(query),
            TextPromptOutcome::Cancelled => {
                return Err("search query was empty or cancelled".into());
            }
            TextPromptOutcome::Unavailable => {}
        }
    }

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

fn episode_file_stem(anime_title: &str, episode_number: &str) -> String {
    format!(
        "{} Episode {}",
        safe_filename(anime_title),
        safe_filename(episode_number)
    )
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
    fn updating_history_keeps_existing_anime_position_and_appends_new_anime() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "ani-cli-record-history-{}-{nonce}",
            std::process::id()
        ));
        let store = HistoryStore::new(&path);
        store
            .write(&[
                HistoryEntry::new("2", "first-1", "First"),
                HistoryEntry::new("5", "second-2", "Second"),
            ])
            .unwrap();

        record_history(
            &store,
            &Anime::new(
                "first-1",
                "First Updated",
                "https://hianime.at/watch/first-1",
            ),
            &Episode::new("3", None, "https://hianime.at/watch/first-1?ep=1003"),
        )
        .unwrap();
        record_history(
            &store,
            &Anime::new("third-3", "Third", "https://hianime.at/watch/third-3"),
            &Episode::new("1", None, "https://hianime.at/watch/third-3?ep=3001"),
        )
        .unwrap();

        assert_eq!(
            store.read().unwrap(),
            vec![
                HistoryEntry::new("3", "first-1", "First Updated"),
                HistoryEntry::new("5", "second-2", "Second"),
                HistoryEntry::new("1", "third-3", "Third"),
            ]
        );
        for suffix in ["", ".lock"] {
            let _ = std::fs::remove_file({
                let mut name = path.as_os_str().to_os_string();
                name.push(suffix);
                std::path::PathBuf::from(name)
            });
        }
    }

    #[test]
    fn download_only_mode_does_not_require_a_player() {
        assert!(!player_required(true));
        assert!(player_required(false));
    }

    #[test]
    fn flatpak_detection_uses_installed_app_directories() {
        // CI machines usually have neither installation path; the result must
        // be a pure filesystem check and not run the Flatpak command.
        let available = flatpak_mpv_installed();
        assert_eq!(
            available,
            Path::new("/var/lib/flatpak/app/io.mpv.Mpv").is_dir()
                || std::env::var_os("HOME").is_some_and(|home| {
                    PathBuf::from(home)
                        .join(".local/share/flatpak/app/io.mpv.Mpv")
                        .is_dir()
                })
        );
    }

    #[test]
    fn explicit_menu_flags_override_environment_and_terminal_detection() {
        let rofi = ani_cli::cli::args::Cli::parse(["--rofi"]).unwrap();
        let dmenu = ani_cli::cli::args::Cli::parse(["--dmenu"]).unwrap();
        assert_eq!(
            preferred_menu_backend(&rofi, true, Some("fzf"), true, true),
            Some("rofi".to_owned())
        );
        assert_eq!(
            preferred_menu_backend(&dmenu, false, Some("fzf"), false, true),
            Some("dmenu".to_owned())
        );
    }

    #[test]
    fn non_terminal_input_prefers_rofi_then_dmenu_like_the_shell_version() {
        let cli = ani_cli::cli::args::Cli::default();
        assert_eq!(
            preferred_menu_backend(&cli, false, Some("fzf"), true, true),
            Some("rofi".to_owned())
        );
        assert_eq!(
            preferred_menu_backend(&cli, false, Some("fzf"), true, false),
            Some("dmenu".to_owned())
        );
    }

    #[test]
    fn interactive_terminal_uses_configured_menu_backend() {
        let cli = ani_cli::cli::args::Cli::default();
        assert_eq!(
            preferred_menu_backend(&cli, true, Some("fzf"), true, true),
            Some("fzf".to_owned())
        );
        assert_eq!(
            preferred_menu_backend(&cli, true, Some("  "), false, false),
            None
        );
    }

    #[test]
    fn supports_legacy_boolean_environment_flags() {
        assert!(is_enabled(Some("1")));
        assert!(!is_enabled(Some("0")));
        assert!(!is_enabled(Some("true")));
        assert!(!is_enabled(None));
    }

    #[test]
    fn selects_source_from_cli_override_or_environment() {
        use ani_cli::cli::args::SourceOverride::{History, NextEpisode};

        assert_eq!(effective_source(None, None).unwrap(), "search");
        assert_eq!(effective_source(None, Some("history")).unwrap(), "history");
        assert_eq!(effective_source(None, Some("nextep")).unwrap(), "nextep");
        assert_eq!(
            effective_source(Some(History), Some("search")).unwrap(),
            "history"
        );
        assert_eq!(
            effective_source(Some(NextEpisode), Some("history")).unwrap(),
            "nextep"
        );
    }

    #[test]
    fn rejects_invalid_default_source_environment_value() {
        assert!(effective_source(None, Some("bad-value")).is_err());
    }

    #[test]
    fn updater_arguments_pin_repository_branch_and_install_root() {
        let args = updater_arguments(Path::new("/home/user/.cargo"), "develop");
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--branch" && pair[1] == "develop")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--root" && pair[1] == "/home/user/.cargo")
        );
        assert!(args.contains(&"https://github.com/gabrielfranicevich/anime".to_owned()));
        assert!(args.contains(&"--force".to_owned()));
        assert!(!args.contains(&"--locked".to_owned()));
    }

    #[test]
    fn rejects_unsafe_branch_names_for_the_updater() {
        assert!(valid_branch_name("develop"));
        assert!(valid_branch_name("feature/rust-port"));
        assert!(!valid_branch_name("--help"));
        assert!(!valid_branch_name("../main"));
        assert!(!valid_branch_name(""));
    }

    #[test]
    fn download_filename_matches_legacy_episode_stem_format() {
        assert_eq!(
            episode_file_stem("Frieren: Beyond Journey's End", "2.5"),
            "Frieren_ Beyond Journey's End Episode 2.5"
        );
        assert_eq!(episode_file_stem("A/B", "1"), "A_B Episode 1");
    }

    #[test]
    fn filename_replaces_platform_invalid_characters() {
        assert_eq!(safe_filename("A/B: C?"), "A_B_ C_");
    }

    #[test]
    fn history_resume_takes_precedence_over_explicit_episode_range() {
        let episodes = vec![
            Episode::new("1", None, "one"),
            Episode::new("2", None, "two"),
            Episode::new("3", None, "three"),
        ];
        let selected = select_fixed_episodes(&episodes, Some("1-2"), Some("2"))
            .unwrap()
            .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|episode| episode.number.as_str())
                .collect::<Vec<_>>(),
            vec!["3"]
        );
    }

    #[test]
    fn explicit_episode_range_is_used_when_not_resuming_history() {
        let episodes = vec![
            Episode::new("1", None, "one"),
            Episode::new("2", None, "two"),
            Episode::new("3", None, "three"),
        ];
        let selected = select_fixed_episodes(&episodes, Some("1-2"), None)
            .unwrap()
            .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|episode| episode.number.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        assert!(
            select_fixed_episodes(&episodes, None, None)
                .unwrap()
                .is_none()
        );
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
    fn multi_episode_selection_is_sorted_and_deduplicated() {
        assert_eq!(
            ordered_episode_indices(vec![3, 1, 3, 0, 1, 2]),
            vec![0, 1, 2, 3]
        );
        assert!(ordered_episode_indices(Vec::new()).is_empty());
    }

    #[test]
    fn quality_menu_lists_best_worst_and_unique_resolutions() {
        let available = vec![
            "720p".to_owned(),
            "1080p".to_owned(),
            "720p".to_owned(),
            "360p".to_owned(),
        ];
        assert_eq!(
            quality_choices(&available),
            vec![
                ("best".to_owned(), "best".to_owned()),
                ("worst".to_owned(), "worst".to_owned()),
                ("1080p".to_owned(), "1080p".to_owned()),
                ("720p".to_owned(), "720p".to_owned()),
                ("360p".to_owned(), "360p".to_owned()),
            ]
        );
    }

    #[test]
    fn finds_next_episode_after_last_watched_episode() {
        let episodes = vec![
            Episode::new("1", None, "one"),
            Episode::new("2", None, "two"),
            Episode::new("3", None, "three"),
        ];
        assert_eq!(
            next_episode_after(&episodes, "2").map(|episode| episode.number.as_str()),
            Some("3")
        );
    }

    #[test]
    fn does_not_resume_finished_or_unmatched_history_entries() {
        let episodes = vec![
            Episode::new("1", None, "one"),
            Episode::new("2", None, "two"),
        ];
        assert_eq!(next_episode_after(&episodes, "2"), None);
        assert_eq!(next_episode_after(&episodes, "99"), None);
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
            selected
                .iter()
                .map(|episode| episode.number.as_str())
                .collect::<Vec<_>>(),
            vec!["3", "1"]
        );
    }
}
