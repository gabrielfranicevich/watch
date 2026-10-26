use std::process::ExitCode;

fn main() -> ExitCode {
    match ani_cli::cli::args::Cli::parse(std::env::args().skip(1)) {
        Ok(cli) if cli.help => {
            print!("{}", ani_cli::cli::args::HELP);
            ExitCode::SUCCESS
        }
        Ok(cli) if cli.version => {
            println!("ani-cli {}", ani_cli::version());
            ExitCode::SUCCESS
        }
        Ok(cli) if cli.query.is_none() && !cli.continue_history && !cli.delete_history && !cli.logview && !cli.update => {
            print!("{}", ani_cli::cli::args::HELP);
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("This Rust build is a migration scaffold; the requested workflow is not implemented yet.");
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("ani-cli: {error}");
            eprintln!("Try 'ani-cli --help' for usage.");
            ExitCode::from(2)
        }
    }
}
