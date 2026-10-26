use std::io::{self, Write};
use std::process::ExitCode;

use ani_cli::cli::selection::read_selection;
use ani_cli::providers::hianime::HiAnimeProvider;

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
    if cli.delete_history {
        return Err("history deletion is not implemented yet".into());
    }
    if cli.logview || cli.update || cli.continue_history || cli.next_episode_countdown {
        return Err("this option is not implemented in the Rust migration yet".into());
    }

    let query = cli.query.as_deref().ok_or("a search query is required")?;
    let provider = HiAnimeProvider::new();
    eprintln!("Searching HiAnime for: {query}");
    let results = provider.search(query)?;
    if results.is_empty() {
        return Err(format!("no results found for '{query}'").into());
    }

    let anime = if let Some(index) = cli.select_nth.checked_sub(1) {
        results.get(index).cloned().ok_or("anime selection is out of range")?
    } else if results.len() == 1 {
        results[0].clone()
    } else {
        for (index, anime) in results.iter().enumerate() {
            println!("{:>2}. {}", index + 1, anime.title);
        }
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut output = io::stdout().lock();
        let index = read_selection(&mut input, &mut output, results.len())?
            .ok_or("selection cancelled")?;
        results[index].clone()
    };

    println!("{}\n{}", anime.title, anime.url);
    let episodes = provider.episodes(&anime)?;
    if episodes.is_empty() {
        return Err(format!("no episodes found for '{}'", anime.title).into());
    }

    if let Some(range) = cli.episode_range.as_deref() {
        let requested = parse_episode_range(range)?;
        let selected: Vec<_> = episodes
            .iter()
            .filter(|episode| requested.iter().any(|number| number == &episode.number))
            .collect();
        if selected.is_empty() {
            return Err(format!("episode range '{range}' did not match available episodes").into());
        }
        for episode in selected {
            println!("Episode {}\t{}", episode.number, episode.url);
        }
    } else {
        for (index, episode) in episodes.iter().enumerate() {
            println!("{:>3}. Episode {}\t{}", index + 1, episode.number, episode.url);
        }
    }

    if cli.download {
        return Err("download mode is not implemented yet".into());
    }
    if cli.vlc || cli.syncplay || cli.skip_intro || cli.exit_after_play || cli.no_detach || cli.quality.is_some() || cli.dubbed {
        eprintln!("Note: player/quality options are parsed but playback is not implemented yet.");
        io::stdout().flush()?;
    }
    Ok(())
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
