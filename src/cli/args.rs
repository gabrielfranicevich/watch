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
    args.get(index)
        .filter(|value| !value.starts_with('-') || *value == "-1")
        .ok_or_else(|| ParseError(format!("{option} requires a value")))
}

fn required_value<'a>(
    args: &'a [String],
    index: usize,
    option: &str,
) -> Result<&'a String, ParseError> {
    args.get(index)
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| ParseError(format!("{option} requires a value")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_and_options_in_either_order() {
        let cli = Cli::parse(["--download", "Frieren", "--episode", "5-6"]).unwrap();
        assert_eq!(cli.query.as_deref(), Some("Frieren"));
        assert!(cli.download);
        assert_eq!(cli.episode_range.as_deref(), Some("5-6"));
    }

    #[test]
    fn joins_multiword_query() {
        let cli = Cli::parse(["blue", "lock", "-q", "720p"]).unwrap();
        assert_eq!(cli.query.as_deref(), Some("blue lock"));
        assert_eq!(cli.quality.as_deref(), Some("720p"));
    }

    #[test]
    fn rejects_unknown_options() {
        let error = Cli::parse(["--not-real"]).unwrap_err();
        assert!(error.to_string().contains("unknown option"));
    }

    #[test]
    fn rejects_missing_option_values() {
        assert!(
            Cli::parse(["--episode"])
                .unwrap_err()
                .to_string()
                .contains("requires a value")
        );
    }

    #[test]
    fn parses_last_episode_sentinel() {
        assert_eq!(
            Cli::parse(["--episode", "-1"])
                .unwrap()
                .episode_range
                .as_deref(),
            Some("-1")
        );
    }

    #[test]
    fn rejects_conflicting_interactive_menus() {
        assert!(Cli::parse(["--rofi", "--dmenu"]).is_err());
    }

    #[test]
    fn last_source_flag_wins_when_continue_and_countdown_are_both_given() {
        let countdown = Cli::parse(["--continue", "--nextep-countdown"]).unwrap();
        assert_eq!(countdown.source_override, Some(SourceOverride::NextEpisode));

        let history = Cli::parse(["--nextep-countdown", "--continue"]).unwrap();
        assert_eq!(history.source_override, Some(SourceOverride::History));
    }

    #[test]
    fn parses_one_based_select_nth_index() {
        let cli = Cli::parse(["-S", "2", "frieren"]).unwrap();
        assert!(cli.select_nth);
        assert_eq!(cli.select_index, Some(2));
        assert_eq!(cli.query.as_deref(), Some("frieren"));
    }

    #[test]
    fn rejects_invalid_select_nth_index() {
        assert!(Cli::parse(["--select-nth"]).is_err());
        assert!(Cli::parse(["--select-nth", "0"]).is_err());
        assert!(Cli::parse(["--select-nth", "abc"]).is_err());
    }

    #[test]
    fn supports_help_and_version_flags() {
        assert!(Cli::parse(["--help"]).unwrap().help);
        assert!(Cli::parse(["-V"]).unwrap().version);
    }

    #[test]
    fn parses_inline_terminal_playback_and_alias() {
        assert!(Cli::parse(["--inline", "Frieren"]).unwrap().inline_playback);
        assert!(Cli::parse(["Frieren", "--here"]).unwrap().inline_playback);
    }

    #[test]
    fn rejects_inline_playback_conflicts() {
        assert!(Cli::parse(["--inline", "--download"]).is_err());
        assert!(Cli::parse(["--inline", "--vlc"]).is_err());
        assert!(Cli::parse(["--syncplay", "--here"]).is_err());
    }
    #[test]
    fn preserves_legacy_operational_flags() {
        let cli = Cli::parse([
            "--delete",
            "--logview",
            "--skip",
            "--no-detach",
            "--exit-after-play",
        ])
        .unwrap();

        assert!(cli.delete_history);
        assert!(cli.logview);
        assert!(cli.skip_intro);
        assert!(cli.no_detach);
        assert!(cli.exit_after_play);
    }

    #[test]
    fn short_episode_range_alias_matches_long_option() {
        let long = Cli::parse(["--episode", "2-4"]).unwrap();
        let short = Cli::parse(["-r", "2-4"]).unwrap();

        assert_eq!(long.episode_range, short.episode_range);
    }

    #[test]
    fn rejects_non_positive_select_nth_values() {
        for value in ["0", "-1", "not-a-number"] {
            assert!(
                Cli::parse(["--select-nth", value]).is_err(),
                "expected --select-nth {value} to be rejected"
            );
        }
    }

    #[test]
    fn keeps_multiword_query_when_options_are_interspersed() {
        let cli = Cli::parse([
            "Frieren",
            "--download",
            "Beyond",
            "--quality",
            "720p",
            "Journey",
        ])
        .unwrap();

        assert_eq!(cli.query.as_deref(), Some("Frieren Beyond Journey"));
        assert_eq!(cli.quality.as_deref(), Some("720p"));
        assert!(cli.download);
    }
}
