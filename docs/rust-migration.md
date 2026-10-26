# Rust migration notes

The Rust port is developed on the `develop` branch. The shell implementation on `main` remains unchanged until feature parity and release validation are complete.

## Build and run

Requires stable Rust and HiAnime network access. Linux and Windows playback prefer MPV and fall back to VLC when available; Linux can use Flatpak MPV. macOS prefers IINA, then MPV/VLC. Termux/Android opens the MPV or VLC app through `am start`; MPV referrer/audio/subtitle options are written to `/storage/emulated/0/mpv/mpv.config.mp4` when storage access is available. Syncplay and intro skipping require `syncplay` and `ani-skip`. Downloads use `yt-dlp` when available for a single media source and fall back to `ffmpeg`; separate audio renditions are muxed with FFmpeg.

```sh
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo fmt --all -- --check
cargo run --release -- "Frieren"
cargo run --release -- --download --episode 1-3 "Frieren"
cargo run --release -- --continue
cargo run --release -- --nextep-countdown "Frieren"
```

## Compatibility settings

- `ANI_CLI_DEFAULT_SOURCE=search|history|nextep`: default source/action when no source flag is specified.
- `ANI_CLI_MODE=sub|dub`: default language mode.
- `ANI_CLI_QUALITY=best|worst|1080p|720p...`: default stream quality.
- `ANI_CLI_EXACT=1`: require a normalized exact title match.
- `ANI_CLI_MENU=fzf|rofi|dmenu`: selection backend; `--rofi` and `--dmenu` override it.
- `ANI_CLI_PLAYER_FLAGS`: extra player or downloader arguments.
- `ANI_CLI_PLAYER=mpv|vlc|syncplay|iina|/path/to/player`: player selection; paths to named players preserve their corresponding argument conventions, and other programs receive the URL plus configured flags.
- `ANI_CLI_NO_DETACH=1`: run the player attached to the terminal.
- `ANI_CLI_EXIT_AFTER_PLAY=1`: return the selected player's exit status after playback.
- `ANI_CLI_SKIP_INTRO=1`: enable ani-skip for supported mpv/Syncplay playback.
- `ANI_CLI_MENU_FLAGS`: additional arguments for the selected external menu.
- `ANI_CLI_DOWNLOAD_DIR`: output directory for downloaded episodes.
- `ANI_CLI_HIST_DIR`: directory for the compatible `ani-hsts` history file.
- `ANI_CLI_LOG=0`: disable system logger records.

## Pending compatibility

`--update` installs from `https://github.com/gabrielfranicevich/anime` using `cargo install --force --git ... --branch ... --root ...`. It requires Cargo, network access, and an installation path whose parent is named `bin` (the normal Cargo layout). The updater validates branch names before invoking Cargo. The checked-in `Cargo.lock` has not yet been regenerated with the full dependency graph, so the updater deliberately does not pass `--locked`; regenerate and validate the lockfile before restoring locked installs. Windows may reject replacement of a currently running executable; close the program and retry or reinstall manually. `--nextep-countdown` queries AnimeSchedule and depends on the provider live API/page structure..

## Current Rust feature surface

The current port contains these workflows:

- HiAnime title search and episode-list parsing, including local fixture regression tests.
- Interactive selection and explicit episode ranges, including history-based continuation.
- HLS stream resolution, quality selection, external audio renditions and subtitle resolution.
- MPV, VLC, Syncplay, IINA, Flatpak MPV and Android playback adapters, subject to tools/platform support.
- Downloads through `yt-dlp` with FFmpeg fallback for a single stream, and FFmpeg muxing when audio is separate.
- Compatible `ani-hsts` history reading, append/update behavior, legacy `.v4` backup, log viewing and Cargo-based update flow.

These are implemented code paths, not a guarantee that the live provider or every external player is always available. Provider markup, anti-bot responses, streaming hosts and third-party tools can change independently of the client.

## Compatibility and validation policy

Keep the original shell entry point on `main` until the Rust branch passes a clean-checkout CI run and the required compatibility checklist. In particular, compare option aliases and argument ordering, environment-variable precedence, exit codes, filenames, history migration, player arguments, subtitles, download behavior and platform-specific defaults. The `--logview` option is implemented and should remain covered by CLI regression tests.

The Rust CI workflow checks formatting, all targets, Clippy with warnings denied, tests, and CLI help/version smoke tests on Linux, macOS and Windows. Network-dependent tests must use local fixtures rather than the live HiAnime site.

## Dependency and updater notes

Cargo resolves Rust dependencies during install/build. Do not add `--locked` to release or updater commands until a complete, committed lockfile has been generated and verified against `Cargo.toml`. The updater uses a validated branch argument and passes every argument to `cargo` separately, not through a shell. It does not independently verify a publisher signature for the resulting executable; treat updates as code execution from the selected Git branch.
