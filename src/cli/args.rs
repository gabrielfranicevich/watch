use std::fmt;

pub const HELP: &str = "ani-cli — browse and watch anime

Usage:
  ani-cli [options] [query]
  ani-cli [query] [options]

Options:
  -c, --continue           Continue watching from history
  -d, --download           Download instead of playing
  -D, --delete             Delete history
  -l, --logview            Show logs
  -s, --syncplay            Use Syncplay
  -S, --select-nth         Select the nth entry
  -q, --quality QUALITY    Specify video quality
  -v, --vlc                Use VLC
  -V, --version            Show version
  -h, --help               Show this help
  -e, --episode RANGE      Select episode(s)
  -r, --range RANGE        Alias for --episode
      --dub                Play dubbed version
      --rofi               Use rofi for selection
      --dmenu              Use dmenu for selection
      --skip               Skip intro (mpv only)
      --no-detach           Keep player attached to terminal
      --exit-after-play    Return the player exit code
  -N, --nextep-countdown   Show countdown to next episode
  -U, --update             Update ani-cli

Current Rust support: searches HiAnime and lists episodes. Playback, downloads, history and several legacy options are still being migrated.
";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub query: Option<String>,
    pub continue_history: bool,
    pub download: bool,
    pub delete_history: bool,
    pub logview: bool,
    pub syncplay: bool,
    pub select_nth: bool,
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
                "-c" | "--continue" => cli.continue_history = true,
                "-d" | "--download" => cli.download = true,
                "-D" | "--delete" => cli.delete_history = true,
                "-l" | "--logview" => cli.logview = true,
                "-s" | "--syncplay" => cli.syncplay = true,
                "-S" | "--select-nth" => cli.select_nth = true,
                "-v" | "--vlc" => cli.vlc = true,
                "-e" | "--episode" | "-r" | "--range" => {
                    index += 1;
                    cli.episode_range = Some(required_value(&args, index, arg)?.clone());
                }
                "-q" | "--quality" => {
                    index += 1;
                    cli.quality = Some(required_value(&args, index, arg)?.clone());
                }
                "-N" | "--nextep-countdown" => cli.next_episode_countdown = true,
                "-U" | "--update" => cli.update = true,
                "--dub" => cli.dubbed = true,
                "--rofi" => cli.rofi = true,
                "--dmenu" => cli.dmenu = true,
                "--skip" => cli.skip_intro = true,
                "--no-detach" => cli.no_detach = true,
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

        if !query_words.is_empty() {
            cli.query = Some(query_words.join(" "));
        }

        Ok(cli)
    }
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
    fn rejects_conflicting_interactive_menus() {
        assert!(Cli::parse(["--rofi", "--dmenu"]).is_err());
    }

    #[test]
    fn supports_help_and_version_flags() {
        assert!(Cli::parse(["--help"]).unwrap().help);
        assert!(Cli::parse(["-V"]).unwrap().version);
    }
}
