# Rust migration notes

The Rust implementation is developed on `main`. Watch is a fork of [pystardust/ani-cli](https://github.com/pystardust/ani-cli); the upstream repository maintains the original shell implementation.

## Build and run

The installed executable and Cargo package are named `watch`. Existing `ANI_CLI_*` environment variables retain their historical names. Install with Cargo when the package is available:

~~~sh
cargo install watch
watch --version
~~~

Otherwise, install the Rust branch from GitHub or build from a checkout:

~~~sh
cargo install --git https://github.com/gabrielfranicevich/watch --branch main

git clone --branch main https://github.com/gabrielfranicevich/watch.git
cd watch
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo fmt --all -- --check

cargo run --release --bin watch -- "Frieren"
cargo run --release --bin watch -- --download --episode 1-3 "Frieren"
cargo run --release --bin watch -- --continue
cargo run --release --bin watch -- --nextep-countdown "Frieren"
~~~

On Linux, `watch` may conflict with the system process-monitoring utility; ensure Cargo’s binary directory is first on `PATH` or invoke `~/.cargo/bin/watch` explicitly. Player selection depends on the platform and which executables are available. On Windows, watch checks for MPV and falls back to VLC when MPV is unavailable; if neither is available, install one or set `ANI_CLI_PLAYER` to an executable path. Linux can use Flatpak MPV when regular MPV is unavailable. macOS can use IINA, MPV, or VLC when installed. Termux/Android playback uses Android app integration and depends on the installed player. Syncplay and intro skipping require their respective external tools.

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

## Catalog language

Use `--language english` (also `en`, `ingles`, `inglés`) or `--language spanish` (also `es`, `espanol`, `español`) to filter catalog and history choices. Matching ignores case. This is separate from `--dub`, which selects dubbed audio.

## Known limitations

- HiAnime markup, streaming hosts, and anti-bot responses can change independently of this client. CI tests use local fixtures and do not establish live-provider availability.
- Playback and downloads depend on external programs and platform-specific behavior. Install those tools separately and ensure they are accessible to the process.
- Inline rendering depends on the selected MPV video output and support for its graphics protocol in the terminal emulator. An available MPV driver does not prove terminal support; if frames do not appear, use regular external playback.
- The updater invokes Cargo to install from a selected Git branch. It requires Cargo and network access, validates the branch name, and passes arguments directly instead of invoking a shell. Windows may reject replacement of a currently running executable; close the application and retry or reinstall manually.
- The updater intentionally does not pass `--locked` yet. Before adding that flag to updater or installation instructions, verify locked dependency resolution from a clean checkout with `cargo check --all-targets --locked` and `cargo test --all-targets --locked`. Regenerate `Cargo.lock` only if Cargo reports that it needs an update.

## Related documentation

- [User tutorial](tutorial.md)
- [Android / Termux guide](android.md)
- [Development guide](development.md)
- [Project roadmap](roadmap.md)
- [Root README](../README.md)
