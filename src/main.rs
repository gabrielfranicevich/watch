use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use ani_cli::cli::selection::read_selection;
use ani_cli::domain::episode::Episode;
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
    if cli.delete_history || cli.logview || cli.update || cli.continue_history || cli.next_episode_countdown {
        return Err("this option is not implemented in the Rust migration yet".into());
    }
    if cli.syncplay || cli.skip_intro {
        return Err("--syncplay and --skip are not implemented in the Rust migration yet".into());
    }

    let query = cli.query.as_deref().ok_or("a search query is required")?;
    let provider = HiAnimeProvider::new();
    eprintln!("Searching HiAnime for: {query}");
    let results = provider.search(query)?;
    if results.is_empty() {
        return Err(format!("no results found for '{query}'").into());
    }

    let anime = if results.len() == 1 || cli.select_nth {
        let index = if results.len() == 1 {
            0
        } else {
            read_index(results.len())?
        };
        results[index].clone()
    } else {
        for (index, anime) in results.iter().enumerate() {
            println!("{:>2}. {}", index + 1, anime.title);
        }
        results[read_index(results.len())?].clone()
    };

    println!("{}\n{}", anime.title, anime.url);
    let episodes = provider.episodes(&anime)?;
    if episodes.is_empty() {
        return Err(format!("no episodes found for '{}'", anime.title).into());
    }

    let selected: Vec<Episode> = if let Some(range) = cli.episode_range.as_deref() {
        let requested = parse_episode_range(range)?;
        let selected: Vec<_> = episodes.into_iter()
            .filter(|episode| requested.iter().any(|number| number == &episode.number))
            .collect();
        if selected.is_empty() {
            return Err(format!("episode range '{range}' did not match available episodes").into());
        }
        selected
    } else {
        for (index, episode) in episodes.iter().enumerate() {
            println!("{:>3}. Episode {}{}", index + 1, episode.number,
                episode.title.as_deref().map(|title| format!(" — {title}")).unwrap_or_default());
        }
        let index = read_index(episodes.len())?;
        vec![episodes[index].clone()]
    };

    let resolver = HiAnimeStreamResolver::new();
    let player = Player::from_vlc_flag(cli.vlc);
    for episode in &selected {
        eprintln!("Resolving episode {} ({})...", episode.number, if cli.dubbed { "dub" } else { "sub" });
        let media = resolver.resolve(episode, cli.dubbed, cli.quality.as_deref().unwrap_or("best"))?;
        let title = format!("{} - Episode {}", anime.title, episode.number);
        if cli.download {
            let filename = format!("{} - Episode {}.mp4", safe_filename(&anime.title), safe_filename(&episode.number));
            let destination = PathBuf::from(filename);
            let status = FfmpegDownloader::download(&media.stream, &destination)?;
            if !status.success() {
                return Err(format!("ffmpeg exited with status {status}").into());
            }
            if let Some(subtitle) = media.subtitle {
                let subtitle_path = destination.with_extension("vtt");
                let response = reqwest::blocking::get(&subtitle.url)?.error_for_status()?;
                std::fs::write(subtitle_path, response.bytes()?)?;
            }
        } else {
            let status = player.play(&media.stream, media.subtitle.as_ref(), &title)?;
            if !status.success() {
                return Err(format!("{} exited with status {status}", player.command_name()).into());
            }
        }
    }
    Ok(())
}

fn read_index(count: usize) -> Result<usize, Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    read_selection(&mut input, &mut output, count)?.ok_or_else(|| "selection cancelled".into())
}

fn safe_filename(value: &str) -> String {
    value.chars().map(|character| {
        if character.is_control() || ['<', '>', ':', '"', '/', '\\', '|', '?', '*'].contains(&character) {
            '_'
        } else {
            character
        }
    }).collect::<String>().trim().to_owned()
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
}
