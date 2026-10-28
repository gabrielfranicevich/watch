use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};

#[cfg(test)]
use ani_cli::application::select_episode_range;
use ani_cli::application::{
  effective_source, next_episode_after, normalize_title, select_fixed_episodes,
};
use ani_cli::cli::menu::{self, MenuOutcome, TextPromptOutcome};
use ani_cli::cli::selection::{BackRequested, SelectionOutcome, read_paged_selection};
use ani_cli::domain::anime::Anime;
use ani_cli::domain::episode::Episode;
use ani_cli::domain::history::HistoryEntry;
use ani_cli::infrastructure::history_store::HistoryStore;
use ani_cli::infrastructure::paths;
use ani_cli::media::downloader::{MediaDownloader, SubtitleDownloader};
use ani_cli::media::player::Player;
use ani_cli::media::terminal_video::TerminalVideoBackend;
use ani_cli::providers::anime_schedule::AnimeScheduleProvider;
use ani_cli::providers::catalog::{AnimeOption, ProviderAnime, ProviderCatalog};
use ani_cli::providers::hianime_stream::ResolvedMedia;
use ani_cli::providers::site::{Language as ProviderLanguage, ProviderId};

fn main() -> ExitCode {
  match run() {
    Ok(code) => code,
    Err(error) if is_selection_cancelled(error.as_ref()) => ExitCode::SUCCESS,
    Err(error) => {
      eprintln!("watch: {error}");
      ExitCode::from(1)
    }
  }
}

#[derive(Debug)]
struct SelectionCancelled;

impl std::fmt::Display for SelectionCancelled {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str("selection cancelled")
  }
}

impl std::error::Error for SelectionCancelled {}

fn selection_cancelled() -> Box<dyn std::error::Error> {
  Box::new(SelectionCancelled)
}

fn is_selection_cancelled(error: &(dyn std::error::Error + 'static)) -> bool {
  error.is::<SelectionCancelled>()
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
  let cli = ani_cli::cli::args::Cli::parse(std::env::args().skip(1))?;
  if cli.help {
    print!("{}", ani_cli::cli::args::HELP);
    return Ok(ExitCode::SUCCESS);
  }
  if cli.version {
    println!("watch {}", ani_cli::version());
    return Ok(ExitCode::SUCCESS);
  }
  if cli.inline_playback && (!io::stdin().is_terminal() || !io::stdout().is_terminal()) {
    return Err("--inline requires an interactive terminal on stdin and stdout".into());
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
      .unwrap_or_else(|| "main".to_owned());
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
      eprintln!("watch: warning: could not fetch next-episode countdown: {error}");
    }
    // The original script prints the countdown and then continues into
    // the normal anime search/playback flow using the same query.
    search_query = Some(query);
  }

  let catalog = ProviderCatalog::new();
  let mut active_source = if source == "history" {
    "history".to_owned()
  } else {
    "search".to_owned()
  };
  let mut pending_query = search_query;
  let mut cached_choices: Option<Vec<AnimeChoice>> = None;

  let (anime, episodes, selected, provider_episodes): (
    Anime,
    Vec<Episode>,
    Vec<Episode>,
    Vec<ProviderEpisodes>,
  ) = loop {
    let choices = if let Some(cached) = cached_choices.take() {
      cached
    } else if active_source == "history" {
      let store = history
        .as_ref()
        .ok_or("cannot determine history path; set ANI_CLI_HIST_DIR or HOME")?;
      history_anime_choices(&catalog, store, cli.language)?
    } else {
      let query = match pending_query.take() {
        Some(query) => query,
        None => read_search_query(&cli)?,
      };
      search_anime_choices(&catalog, &query, cli.language)?
    };

    if choices.is_empty() {
      return Err("no anime choices available".into());
    }

    let labels: Vec<String> = choices.iter().map(|choice| choice.label.clone()).collect();
    let selected_index = match select_item(&labels, "Select anime: ", cli.select_index, &cli) {
      Ok(index) => index,
      Err(error) if is_back_requested(error.as_ref()) => {
        active_source = "search".to_owned();
        pending_query = None;
        cached_choices = None;
        continue;
      }
      Err(error) => return Err(error),
    };

    let choice = choices[selected_index].clone();
    println!("{}", choice.anime.title);

    let source_options = if choice.sources.is_empty() {
      vec![ProviderAnime {
        provider: choice.provider,
        anime: choice.anime.clone(),
      }]
    } else {
      choice.sources.clone()
    };
    let mut provider_episodes = Vec::new();
    for source in source_options {
      let loaded = if source.provider == choice.provider && source.anime.id == choice.anime.id {
        choice
          .prefetched_episodes
          .clone()
          .map(Ok)
          .unwrap_or_else(|| catalog.episodes(&source))
      } else {
        catalog.episodes(&source)
      };
      match loaded {
        Ok(episodes) if !episodes.is_empty() => {
          provider_episodes.push(ProviderEpisodes { source, episodes });
        }
        Ok(_) => eprintln!(
          "watch: warning: {} returned no episodes for '{}'",
          source.provider.name(),
          source.anime.title
        ),
        Err(error) => eprintln!(
          "watch: warning: could not load episodes from {}: {error}",
          source.provider.name()
        ),
      }
    }
    if provider_episodes.is_empty() {
      return Err(format!("no providers exposed episodes for '{}'", choice.anime.title).into());
    }
    let episodes = merge_provider_episodes(&provider_episodes);
    if episodes.is_empty() {
      return Err(format!("no episodes found for '{}'", choice.anime.title).into());
    }

    let selected_episodes = if let Some(selected) = select_fixed_episodes(
      &episodes,
      cli.episode_range.as_deref(),
      choice.resume_episode.as_deref(),
    )? {
      selected
    } else {
      let episode_labels: Vec<String> = episodes
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

      let indices = match select_items(&episode_labels, "Select episode: ", None, true, &cli) {
        Ok(indices) => indices,
        Err(error) if is_back_requested(error.as_ref()) => {
          if choices.len() > 1 {
            cached_choices = Some(choices);
          } else {
            // With one result, returning to its list would immediately
            // select it again, so return to the search prompt instead.
            active_source = "search".to_owned();
            pending_query = None;
            cached_choices = None;
          }
          continue;
        }
        Err(error) => return Err(error),
      };

      indices
        .into_iter()
        .map(|index| episodes[index].clone())
        .collect()
    };

    break (choice.anime, episodes, selected_episodes, provider_episodes);
  };
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
    Player::Mpv
  } else if cli.inline_playback {
    select_terminal_player(
      env_player.as_deref(),
      use_flatpak_default,
      command_available("mpv"),
    )?
  } else if cli.syncplay || cli.vlc || env_player.is_some() || cfg!(target_os = "android") {
    Player::from_configuration(cli.vlc, cli.syncplay, false, env_player.as_deref())
  } else {
    select_default_player(
      use_iina_default,
      command_available("iina"),
      use_flatpak_default,
      command_available("mpv"),
      command_available("vlc"),
    )?
  };
  let mut player_flags: Vec<String> = std::env::var("ANI_CLI_PLAYER_FLAGS")
    .unwrap_or_default()
    .split_whitespace()
    .map(str::to_owned)
    .collect();
  if cli.inline_playback {
    let available_outputs = playback_output_help(&player)?;
    player_flags.extend(TerminalVideoBackend::from_environment()?.mpv_flags(&available_outputs)?);
  }
  let download_dir = std::env::var_os("ANI_CLI_DOWNLOAD_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("."));

  if cli.download {
    std::fs::create_dir_all(&download_dir)?;
  }

  let exit_after_play = cli.exit_after_play || env_enabled("ANI_CLI_EXIT_AFTER_PLAY");
  let attached =
    cli.inline_playback || cli.no_detach || env_enabled("ANI_CLI_NO_DETACH") || exit_after_play;
  let mut playback = PlaybackConfig {
    catalog: &catalog,
    provider_episodes: &provider_episodes,
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

#[derive(Debug, Clone)]
struct ProviderEpisodes {
  source: ProviderAnime,
  episodes: Vec<Episode>,
}

struct PlaybackConfig<'a> {
  catalog: &'a ProviderCatalog,
  provider_episodes: &'a [ProviderEpisodes],
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
    let (source, media) = self.resolve_episode(episode)?;
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
          eprintln!("watch: warning: could not download subtitles: {error}");
        }
      }

      let status =
        MediaDownloader::download_with_extra(&media.stream, &destination, &self.player_flags)?;
      ensure_success(status, "ffmpeg")?;
      self.record(&source, episode)?;
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
      if let Some(driver) = self
        .player_flags
        .iter()
        .find_map(|flag| flag.strip_prefix("--vo="))
      {
        eprintln!(
          "Starting inline playback with {} using video output {driver}...",
          self.player.command_name()
        );
      }
      let status = self.player.play_with_options(
        &media.stream,
        media.subtitle.as_ref(),
        &title,
        &self.player_flags,
        &media_flags,
      )?;
      self.record(&source, episode)?;
      Ok(PlaybackResult::Attached(status))
    } else {
      let child = self.player.launch_detached_with_options(
        &media.stream,
        media.subtitle.as_ref(),
        &title,
        &self.player_flags,
        &media_flags,
      )?;
      self.record(&source, episode)?;
      Ok(PlaybackResult::Detached(child))
    }
  }

  fn resolve_episode(
    &self,
    episode: &Episode,
  ) -> Result<(ProviderAnime, ResolvedMedia), Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    let mut failures = Vec::new();

    for (provider_order, entry) in self.provider_episodes.iter().enumerate() {
      let Some(source_episode) = entry
        .episodes
        .iter()
        .find(|candidate| same_episode_number(&candidate.number, &episode.number))
      else {
        continue;
      };
      match self.catalog.resolver(entry.source.provider).resolve(
        source_episode,
        self.dubbed,
        &self.quality,
      ) {
        Ok(media) => candidates.push((provider_order, entry.source.clone(), media)),
        Err(error) => failures.push(format!("{}: {error}", entry.source.provider.name())),
      }
    }

    candidates.sort_by(|left, right| {
      compare_quality_fit(&left.2, &right.2, &self.quality).then_with(|| left.0.cmp(&right.0))
    });

    if let Some((_, source, media)) = candidates.into_iter().next() {
      return Ok((source, media));
    }

    let detail = if failures.is_empty() {
      "no provider has that episode".to_owned()
    } else {
      failures.join("; ")
    };
    Err(format!("could not resolve episode {}: {detail}", episode.number).into())
  }

  fn record(
    &self,
    source: &ProviderAnime,
    episode: &Episode,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(store) = self.history {
      record_history_for_provider(store, &source.anime, episode, source.provider)?;
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
        PlaybackResult::Attached(status) => ensure_success(status, playback.player.command_name())?,
        PlaybackResult::Downloaded => return Ok(ExitCode::SUCCESS),
      }
    }
  }
}

fn same_episode_number(left: &str, right: &str) -> bool {
  match (left.parse::<f64>(), right.parse::<f64>()) {
    (Ok(left), Ok(right)) => left == right,
    _ => left == right,
  }
}

fn episode_number_sort_key(number: &str) -> f64 {
  number.parse::<f64>().unwrap_or(f64::MAX)
}

fn merge_provider_episodes(sources: &[ProviderEpisodes]) -> Vec<Episode> {
  let mut episodes: Vec<Episode> = Vec::new();
  for source in sources {
    for episode in &source.episodes {
      if !episodes
        .iter()
        .any(|existing| same_episode_number(&existing.number, &episode.number))
      {
        episodes.push(episode.clone());
      }
    }
  }
  episodes.sort_by(|left, right| {
    episode_number_sort_key(&left.number)
      .total_cmp(&episode_number_sort_key(&right.number))
      .then_with(|| left.number.cmp(&right.number))
  });
  episodes
}

fn media_quality_rank(media: &ResolvedMedia) -> u32 {
  media
    .qualities
    .iter()
    .filter_map(|quality| quality.value.trim_end_matches('p').parse::<u32>().ok())
    .max()
    .unwrap_or_default()
}

fn resolved_media_quality(media: &ResolvedMedia) -> Option<u32> {
  media
    .stream
    .quality
    .as_deref()
    .map(quality_height)
    .filter(|height| *height > 0)
    .or_else(|| {
      let available = media_quality_rank(media);
      (available > 0).then_some(available)
    })
}

fn compare_quality_fit(
  left: &ResolvedMedia,
  right: &ResolvedMedia,
  requested: &str,
) -> std::cmp::Ordering {
  let left_quality = resolved_media_quality(left);
  let right_quality = resolved_media_quality(right);
  if requested == "worst" {
    return match (left_quality, right_quality) {
      (Some(left), Some(right)) => left.cmp(&right),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => std::cmp::Ordering::Equal,
    };
  }

  let target = quality_height(requested);
  if target > 0 {
    return match (left_quality, right_quality) {
      (Some(left), Some(right)) => left
        .abs_diff(target)
        .cmp(&right.abs_diff(target))
        .then_with(|| right.cmp(&left)),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => std::cmp::Ordering::Equal,
    };
  }

  match (left_quality, right_quality) {
    (Some(left), Some(right)) => right.cmp(&left),
    (Some(_), None) => std::cmp::Ordering::Less,
    (None, Some(_)) => std::cmp::Ordering::Greater,
    (None, None) => std::cmp::Ordering::Equal,
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

  eprintln!("Updating watch from branch '{branch}'...");
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
  eprintln!("watch update completed.");
  Ok(())
}

fn updater_arguments(install_root: &Path, branch: &str) -> Vec<OsString> {
  vec![
    OsString::from("install"),
    OsString::from("--force"),
    OsString::from("--git"),
    OsString::from("https://github.com/gabrielfranicevich/watch"),
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

#[cfg(test)]
fn record_history(store: &HistoryStore, anime: &Anime, episode: &Episode) -> io::Result<()> {
  record_history_for_provider(store, anime, episode, ProviderId::HiAnime)
}

fn record_history_for_provider(
  store: &HistoryStore,
  anime: &Anime,
  episode: &Episode,
  provider: ProviderId,
) -> io::Result<()> {
  let mut entries = store.read()?;
  let provider_key = provider.key();
  if let Some(existing) = entries.iter_mut().find(|entry| {
    entry.anime_id == anime.id && entry.provider.as_deref().unwrap_or("hianime") == provider_key
  }) {
    existing.episode_number = episode.number.clone();
    existing.anime_title = anime.title.clone();
  } else {
    let entry = if provider == ProviderId::HiAnime {
      HistoryEntry::new(
        episode.number.clone(),
        anime.id.clone(),
        anime.title.clone(),
      )
    } else {
      HistoryEntry::with_provider(
        episode.number.clone(),
        anime.id.clone(),
        anime.title.clone(),
        provider_key,
      )
    };
    entries.push(entry);
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
  let value = value
    .split_once('x')
    .map(|(_, height)| height)
    .unwrap_or(value);
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

#[derive(Debug, Clone)]
struct AnimeChoice {
  label: String,
  anime: Anime,
  provider: ProviderId,
  sources: Vec<ProviderAnime>,
  resume_episode: Option<String>,
  prefetched_episodes: Option<Vec<Episode>>,
}

fn search_anime_choices(
  catalog: &ProviderCatalog,
  query: &str,
  language: Option<ani_cli::cli::args::Language>,
) -> Result<Vec<AnimeChoice>, Box<dyn std::error::Error>> {
  let mut results = catalog.search_in_language(query, language.map(provider_language))?;
  let exact = std::env::var("ANI_CLI_EXACT").is_ok_and(|value| value == "1");
  if exact {
    let normalized_query = normalize_title(query);
    results.retain(|result| {
      result
        .sources
        .iter()
        .any(|source| normalize_title(&source.anime.title) == normalized_query)
    });
  }
  if results.is_empty() {
    let message = if exact {
      format!("no exact match found for '{query}'")
    } else {
      format!("no results found for '{query}'")
    };
    return Err(message.into());
  }

  results
    .into_iter()
    .map(|option: AnimeOption| {
      let primary = option
        .sources
        .first()
        .ok_or("provider catalog returned a title without sources")?;
      Ok(AnimeChoice {
        label: option.label,
        anime: primary.anime.clone(),
        provider: primary.provider,
        sources: option.sources,
        resume_episode: None,
        prefetched_episodes: None,
      })
    })
    .collect()
}

fn history_anime_choices(
  catalog: &ProviderCatalog,
  store: &HistoryStore,
  language: Option<ani_cli::cli::args::Language>,
) -> Result<Vec<AnimeChoice>, Box<dyn std::error::Error>> {
  store.backup_legacy_entries()?;
  let entries = store.read()?;
  if entries.is_empty() {
    return Err("watch history is empty".into());
  }

  let mut recent = Vec::<HistoryEntry>::new();
  for entry in entries.into_iter().rev() {
    if !recent
      .iter()
      .any(|item| item.anime_id == entry.anime_id && item.provider == entry.provider)
    {
      recent.push(entry);
    }
  }

  let mut choices = Vec::new();
  for entry in recent {
    let history_provider = entry
      .provider
      .as_deref()
      .and_then(ProviderId::from_key)
      .unwrap_or(ProviderId::HiAnime);
    if language.is_some_and(|language| history_provider.language() != provider_language(language)) {
      continue;
    }
    let anime = Anime::new(
      entry.anime_id.clone(),
      entry.anime_title.clone(),
      format!(
        "{}/{}",
        history_provider.base_url(),
        entry.anime_id.trim_start_matches('/')
      ),
    );
    let episodes = match catalog.episodes(&ProviderAnime {
      provider: history_provider,
      anime: anime.clone(),
    }) {
      Ok(episodes) => episodes,
      Err(error) => {
        eprintln!(
          "Skipping '{}' in history because its episode list could not be loaded: {error}",
          entry.anime_title
        );
        continue;
      }
    };
    if !episodes
      .iter()
      .any(|episode| episode.number == entry.episode_number)
    {
      eprintln!(
        "Skipping '{}' in history because episode {} is no longer available",
        entry.anime_title, entry.episode_number
      );
      continue;
    }
    let Some(next_episode) = next_episode_after(&episodes, &entry.episode_number) else {
      eprintln!(
        "Skipping '{}' in history because it has no next episode",
        entry.anime_title
      );
      continue;
    };
    choices.push(AnimeChoice {
      label: format!(
        "{} — next episode {}",
        entry.anime_title, next_episode.number
      ),
      anime,
      provider: history_provider,
      sources: Vec::new(),
      resume_episode: Some(entry.episode_number),
      prefetched_episodes: Some(episodes),
    });
  }

  if choices.is_empty() {
    return Err("no unwatched series in history".into());
  }
  Ok(choices)
}

fn provider_language(language: ani_cli::cli::args::Language) -> ProviderLanguage {
  match language {
    ani_cli::cli::args::Language::English => ProviderLanguage::English,
    ani_cli::cli::args::Language::Spanish => ProviderLanguage::Spanish,
  }
}

fn is_back_requested(error: &(dyn std::error::Error + 'static)) -> bool {
  error.downcast_ref::<BackRequested>().is_some()
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

fn single_item_index(labels: &[String]) -> Option<usize> {
  (labels.len() == 1).then_some(0)
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
      return Err(
        format!(
          "selection {number} is outside the valid range 1-{}",
          labels.len()
        )
        .into(),
      );
    }
    return Ok(vec![index]);
  }

  if let Some(index) = single_item_index(labels) {
    let kind = if prompt.to_ascii_lowercase().contains("episode") {
      "episode"
    } else if prompt.to_ascii_lowercase().contains("anime") {
      "anime result"
    } else {
      "option"
    };
    println!(
      "Only one {kind} available; selecting it automatically: {}",
      labels[index]
    );
    return Ok(vec![index]);
  }

  let backend = menu_backend_for_cli(cli);

  match menu::select(labels, prompt, backend.as_deref(), multi)? {
    MenuOutcome::Selected(indices) => {
      let indices = ordered_episode_indices(indices);
      if indices.is_empty() {
        Err(selection_cancelled())
      } else {
        Ok(indices)
      }
    }
    MenuOutcome::Cancelled => Err(selection_cancelled()),
    MenuOutcome::Back => Err(Box::new(BackRequested)),
    MenuOutcome::Unavailable => match read_paged_selection(labels, prompt, multi) {
      Ok(outcome) => match outcome {
        SelectionOutcome::Selected(indices) => {
          let indices = ordered_episode_indices(indices);
          if indices.is_empty() {
            Err(selection_cancelled())
          } else {
            Ok(indices)
          }
        }
        SelectionOutcome::Back => Err(Box::new(BackRequested)),
        SelectionOutcome::Cancelled => Err(selection_cancelled()),
      },
      Err(error) if error.kind() == io::ErrorKind::Unsupported => {
        select_from_numbered_pages(labels, prompt, multi)
      }
      Err(error) => Err(error.into()),
    },
  }
}

fn select_from_numbered_pages(
  labels: &[String],
  prompt: &str,
  multi: bool,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
  let page_count = labels.len().div_ceil(15);
  let mut page = 0;
  loop {
    let start = page * 15;
    let end = (start + 15).min(labels.len());
    for (index, label) in labels.iter().enumerate().take(end).skip(start) {
      println!("{:>2}. {label}", index + 1);
    }
    println!(
      "{prompt}[page {}/{}; enter n/p to change page, q to cancel]",
      page + 1,
      page_count
    );

    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
      return Err(selection_cancelled());
    }
    match input.trim().to_ascii_lowercase().as_str() {
      "n" if page + 1 < page_count => page += 1,
      "p" if page > 0 => page -= 1,
      "q" => return Err(selection_cancelled()),
      value => match parse_page_selection(value, start, end, multi) {
        Some(indices) if !indices.is_empty() => return Ok(ordered_episode_indices(indices)),
        _ => eprintln!("Enter a valid selection from 1 to {}.", labels.len()),
      },
    }
  }
}

fn parse_page_selection(input: &str, start: usize, end: usize, multi: bool) -> Option<Vec<usize>> {
  let tokens: Vec<_> = input.split_whitespace().collect();
  if tokens.is_empty() || (!multi && tokens.len() != 1) {
    return None;
  }
  let mut indices = Vec::new();
  for token in tokens {
    let number = token.parse::<usize>().ok()?;
    if !(start + 1..=end).contains(&number) {
      return None;
    }
    let index = number - 1;
    if !indices.contains(&index) {
      indices.push(index);
    }
  }
  Some(indices)
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

fn select_default_player(
  use_iina_default: bool,
  iina_command_available: bool,
  use_flatpak_default: bool,
  mpv_available: bool,
  vlc_available: bool,
) -> Result<Player, &'static str> {
  if use_iina_default && iina_command_available {
    return Ok(Player::from_configuration(
      false,
      false,
      false,
      Some("iina"),
    ));
  }
  if use_iina_default {
    return Ok(Player::Iina);
  }
  if use_flatpak_default {
    return Ok(Player::FlatpakMpv);
  }
  if mpv_available {
    return Ok(Player::Mpv);
  }
  if vlc_available {
    return Ok(Player::Vlc);
  }

  Err("no supported player found; install MPV or VLC, or set ANI_CLI_PLAYER")
}

fn player_required(download_only: bool) -> bool {
  !download_only
}

fn playback_output_help(player: &Player) -> Result<String, Box<dyn std::error::Error>> {
  Ok(player.terminal_video_output_help()?)
}

fn select_terminal_player(
  configured_player: Option<&str>,
  flatpak_mpv_available: bool,
  mpv_available: bool,
) -> Result<Player, &'static str> {
  if let Some(configured_player) = configured_player.filter(|value| !value.trim().is_empty()) {
    let player = Player::from_configuration(false, false, false, Some(configured_player));
    return if player.supports_terminal_video() {
      Ok(player)
    } else {
      Err("--inline requires MPV; set ANI_CLI_PLAYER to an MPV executable or unset it")
    };
  }

  if flatpak_mpv_available {
    return Ok(Player::FlatpakMpv);
  }
  if mpv_available {
    return Ok(Player::Mpv);
  }

  Err("--inline requires MPV with terminal video output support; install MPV or set ANI_CLI_PLAYER")
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
      let extensions =
        std::env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
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
        return Err(selection_cancelled());
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
    return Err(selection_cancelled());
  }
  Ok(query.to_owned())
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
#[path = "../tests/unit/binary_tests.rs"]
mod tests;
