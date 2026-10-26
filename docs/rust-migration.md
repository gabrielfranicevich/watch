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

`--update` uses `cargo install --force --locked --git ... --branch ...` and therefore requires Cargo, network access, and an installation path whose parent is named `bin` (the normal Cargo layout). Windows may reject replacement of a currently running executable; in that case, update after closing it or reinstall manually. `--nextep-countdown` queries AnimeSchedule and depends on that provider's live API/page structure.