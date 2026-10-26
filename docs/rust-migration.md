# Rust migration notes

The Rust port is developed on the `develop` branch. The shell implementation on `main` remains unchanged until feature parity and release validation are complete.

## Build and run

Requires stable Rust, HiAnime network access, and `mpv` or `vlc` for playback. Syncplay and intro skipping require `syncplay` and `ani-skip`. Downloads use `yt-dlp` when available for a single media source and fall back to `ffmpeg`; separate audio renditions are muxed with FFmpeg.

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

- `ANI_CLI_MODE=sub|dub`: default language mode.
- `ANI_CLI_QUALITY=best|worst|1080p|720p...`: default stream quality.
- `ANI_CLI_EXACT=1`: require a normalized exact title match.
- `ANI_CLI_MENU=fzf|rofi|dmenu`: selection backend; `--rofi` and `--dmenu` override it.
- `ANI_CLI_PLAYER_FLAGS`: extra player arguments.
- `ANI_CLI_PLAYER=mpv|vlc|syncplay`: player selection.
- `ANI_CLI_DOWNLOAD_DIR`: output directory for downloaded episodes.
- `ANI_CLI_HIST_DIR`: directory for the compatible `ani-hsts` history file.
- `ANI_CLI_LOG=0`: disable system logger records.

## Pending compatibility

`--update` uses `cargo install --force --locked --git ... --branch ...` and therefore requires Cargo, network access, and an installation path whose parent is named `bin` (the normal Cargo layout). Windows may reject replacement of a currently running executable; in that case, update after closing it or reinstall manually. `--nextep-countdown` queries AnimeSchedule and depends on that provider's live API/page structure.