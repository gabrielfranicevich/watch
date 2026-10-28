use std::fmt;

pub const HELP: &str = "watch — browse and watch anime

Usage:
  watch [options] [query]
  watch [query] [options]

Options:
  -c, --continue           Continue watching from history
  -d, --download           Download instead of playing
  -D, --delete             Delete history
  -l, --logview            Show logs
  -s, --syncplay            Use Syncplay
  -S, --select-nth N       Select the nth entry
  -q, --quality QUALITY    Specify video quality
  -v, --vlc                Use VLC
  -V, --version            Show version
  -h, --help               Show this help
  -e, --episode RANGE      Select episodes, e.g. 5-6 or \"5 6\"
  -r, --range RANGE        Alias for --episode
      --dub                Play dubbed version
      --rofi               Use rofi for selection
      --dmenu              Use dmenu for selection
      --skip               Skip intro with MPV, Flatpak MPV, or Syncplay
      --no-detach           Keep player attached to terminal
      --inline, --here      Play in the terminal using graphics protocols when available
      --exit-after-play    Return the player exit code
  -N, --nextep-countdown   Show countdown to next episode
  -U, --update             Update watch

watch searches English and Spanish anime providers, merges duplicate titles by language, lets you choose a source, and supports episode selection, playback, downloads, subtitles, watch history, external menu backends, inline terminal video, the Cargo-based updater, and next-episode countdown.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOverride {
  History,
  NextEpisode,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
  pub query: Option<String>,
  pub continue_history: bool,
  pub source_override: Option<SourceOverride>,
  pub download: bool,
  pub delete_history: bool,
  pub logview: bool,
  pub syncplay: bool,
  pub select_nth: bool,
  pub select_index: Option<usize>,
  pub quality: Option<String>,
  pub vlc: bool,
  pub version: bool,
  pub help: bool,
  pub episode_range: Option<String>,
  pub dubbed: bool,
  pub rofi: bool,
  pub dmenu: bool,
  pub skip_intro: bool,
  pub no_detach: bool,
  pub inline_playback: bool,
  pub exit_after_play: bool,
  pub next_episode_countdown: bool,
  pub update: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl std::error::Error for ParseError {}

impl Cli {
  pub fn parse<I, S>(args: I) -> Result<Self, ParseError>
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let mut cli = Self::default();
    let mut query_words = Vec::new();
    let mut index = 0;

    while index < args.len() {
      let arg = &args[index];
      match arg.as_str() {
        "-h" | "--help" => cli.help = true,
        "-V" | "--version" => cli.version = true,
        "-c" | "--continue" => {
          cli.continue_history = true;
          cli.source_override = Some(SourceOverride::History);
        }
        "-d" | "--download" => cli.download = true,
        "-D" | "--delete" => cli.delete_history = true,
        "-l" | "--logview" => cli.logview = true,
        "-s" | "--syncplay" => cli.syncplay = true,
        "-S" | "--select-nth" => {
          index += 1;
          let value = required_value(&args, index, arg)?;
          let number = value
            .parse::<usize>()
            .map_err(|_| ParseError(format!("{arg} requires a positive integer")))?;
          if number == 0 {
            return Err(ParseError(format!("{arg} requires a positive integer")));
          }
          cli.select_nth = true;
          cli.select_index = Some(number);
        }
        "-v" | "--vlc" => cli.vlc = true,
        "-e" | "--episode" | "-r" | "--range" => {
          index += 1;
          cli.episode_range = Some(required_episode_value(&args, index, arg)?.clone());
        }
        "-q" | "--quality" => {
          index += 1;
          cli.quality = Some(required_value(&args, index, arg)?.clone());
        }
        "-N" | "--nextep-countdown" => {
          cli.next_episode_countdown = true;
          cli.source_override = Some(SourceOverride::NextEpisode);
        }
        "-U" | "--update" => cli.update = true,
        "--dub" => cli.dubbed = true,
        "--rofi" => cli.rofi = true,
        "--dmenu" => cli.dmenu = true,
        "--skip" => cli.skip_intro = true,
        "--no-detach" => cli.no_detach = true,
        "--inline" | "--here" => cli.inline_playback = true,
        "--exit-after-play" => cli.exit_after_play = true,
        _ if arg.starts_with('-') => {
          return Err(ParseError(format!("unknown option '{arg}'")));
        }
        _ => query_words.push(arg.clone()),
      }
      index += 1;
    }

    if cli.rofi && cli.dmenu {
      return Err(ParseError(
        "--rofi and --dmenu cannot be used together".into(),
      ));
    }
    if cli.inline_playback && cli.download {
      return Err(ParseError(
        "--inline cannot be combined with --download".into(),
      ));
    }
    if cli.inline_playback && (cli.vlc || cli.syncplay) {
      return Err(ParseError(
        "--inline requires MPV and cannot be combined with --vlc or --syncplay".into(),
      ));
    }

    if !query_words.is_empty() {
      cli.query = Some(query_words.join(" "));
    }

    Ok(cli)
  }
}

fn required_episode_value<'a>(
  args: &'a [String],
  index: usize,
  option: &str,
) -> Result<&'a String, ParseError> {
  args
    .get(index)
    .filter(|value| !value.starts_with('-') || *value == "-1")
    .ok_or_else(|| ParseError(format!("{option} requires a value")))
}

fn required_value<'a>(
  args: &'a [String],
  index: usize,
  option: &str,
) -> Result<&'a String, ParseError> {
  args
    .get(index)
    .filter(|value| !value.starts_with('-'))
    .ok_or_else(|| ParseError(format!("{option} requires a value")))
}

#[cfg(test)]
#[path = "../../tests/unit/cli/args.rs"]
mod tests;
