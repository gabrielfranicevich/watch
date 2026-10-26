# Rust migration notes

The Rust implementation is developed on `develop`. The historical shell implementation remains on `main` as a reference; the two branches are not interchangeable releases.

## Build and run

Install stable Rust 1.85 or newer. From a checkout:

~~~sh
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo fmt --all -- --check

cargo run --release -- "Frieren"
cargo run --release -- --download --episode 1-3 "Frieren"
cargo run --release -- --continue
cargo run --release -- --nextep-countdown "Frieren"
~~~

Player selection depends on the platform and which executables are available. On Windows, ani-cli checks for MPV and falls back to VLC when MPV is unavailable; if neither is available, install one or set `ANI_CLI_PLAYER` to an executable path. Linux can use Flatpak MPV when regular MPV is unavailable. macOS can use IINA, MPV, or VLC when installed. Termux/Android playback uses Android app integration and depends on the installed player. Syncplay and intro skipping require their respective external tools.

Downloads use yt-dlp when available for a compatible single media stream and FFmpeg as a fallback. Separate audio renditions are muxed with FFmpeg.

## Compatibility settings

- `ANI_CLI_DEFAULT_SOURCE=search|history|nextep`: default source/action when no source flag is specified.
- `ANI_CLI_MODE=sub|dub`: default audio mode.
- `ANI_CLI_QUALITY=best|worst|1080p|720p...`: default stream quality.
- `ANI_CLI_EXACT=1`: require a normalized exact title match.
- `ANI_CLI_MENU=fzf|rofi|dmenu`: default selection backend; `--rofi` and `--dmenu` override it.
- `ANI_CLI_MENU_FLAGS`: extra arguments for the menu program.
- `ANI_CLI_PLAYER=mpv|vlc|syncplay|iina|/path/to/player`: player selection.
- `ANI_CLI_PLAYER_FLAGS`: extra player/downloader arguments.
- `ANI_CLI_NO_DETACH=1`: keep the player attached to the terminal.
- `ANI_CLI_EXIT_AFTER_PLAY=1`: return the selected player's exit status.
- `ANI_CLI_SKIP_INTRO=1`: enable intro skipping for supported MPV/Syncplay playback.
- `ANI_CLI_DOWNLOAD_DIR`: download destination.
- `ANI_CLI_HIST_DIR`: directory for the compatible `ani-hsts` history file.
- `ANI_CLI_LOG=0`: disable system logger records.

## Known limitations

- HiAnime markup, streaming hosts, and anti-bot responses can change independently of this client. CI tests use local fixtures and do not establish live-provider availability.
- Playback and downloads depend on external programs and platform-specific behavior. Install those tools separately and ensure they are accessible to the process.
- The updater invokes Cargo to install from a selected Git branch. It requires Cargo and network access, validates the branch name, and passes arguments directly instead of invoking a shell. Windows may reject replacement of a currently running executable; close the application and retry or reinstall manually.
- The checked-in `Cargo.lock` has not yet been regenerated with the complete dependency graph, so updater commands deliberately do not pass `--locked`. Regenerate and verify the lockfile before restoring locked installs.

## Related documentation

- [User tutorial](tutorial.md)
- [Development guide](development.md)
- [Project roadmap](roadmap.md)
- [Root README](../README.md)
