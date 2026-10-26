# ani-cli

Search anime, select episodes, and stream or download them from your terminal.

[![Rust CI](https://github.com/gabrielfranicevich/anime/actions/workflows/rust.yml/badge.svg?branch=develop)](https://github.com/gabrielfranicevich/anime/actions/workflows/rust.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)
![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange)

ani-cli searches HiAnime, resolves available HLS streams, and hands playback or downloads to tools installed on your machine. It does not host video files.

> **Project status:** the Rust implementation is developed on `develop`. Prebuilt binaries and updated third-party packages are not published yet; some package managers may still provide the historical shell version from `main`.

<details>
<summary>Contents</summary>

- [Quick start](#quick-start)
- [Examples](#examples)
- [What it supports](#what-it-supports)
- [Common options](#common-options)
- [Runtime dependencies](#runtime-dependencies)
- [Configuration](#configuration)
- [Caveats](#caveats)
- [Documentation and support](#documentation-and-support)
- [License](#license)

</details>

## Quick start

Install stable Rust 1.85 or newer and Git, then install the CLI and search for a title:

~~~sh
cargo install --git https://github.com/gabrielfranicevich/anime --branch develop
ani-cli "Frieren"
~~~

Choose a result and an episode from the interactive menus. Playback requires a supported media player, such as MPV or VLC, installed and available on `PATH`. On Windows, install the player in the same environment where you run ani-cli. See the [tutorial](docs/tutorial.md) for setup and troubleshooting.

## Examples

~~~sh
# Search and play interactively
ani-cli "Frieren"

# Prefer 720p when the source offers it
ani-cli --quality 720p "Frieren"

# Download episodes 1 through 3
ani-cli --download --episode 1-3 "Frieren"

# Download individual episodes
ani-cli --download --episode "1 3 5" "Frieren"

# Continue from watch history
ani-cli --continue

# Show the next-episode countdown
ani-cli --nextep-countdown "Frieren"
~~~

Run `ani-cli --help` for the complete option list.

## What it supports

- HiAnime title search and episode selection.
- HLS stream resolution, quality selection, external audio, and subtitles.
- Playback through MPV, VLC, Syncplay, IINA, Flatpak MPV, and supported Android player integrations.
- Downloads using yt-dlp when suitable, FFmpeg fallback, and FFmpeg muxing for separate audio/video streams.
- Compatible watch history, continuation, log viewing, external menu backends, and a Cargo-based updater.

These paths depend on the provider, platform, media player, and external tools. An implemented feature does not guarantee that the live provider or every player will always be available.

## Common options

| Option | Purpose |
| --- | --- |
| `-c, --continue` | Continue from watch history. |
| `-d, --download` | Download instead of playing. |
| `-e, --episode RANGE`, `-r, --range RANGE` | Choose ranges like `1-3` or individual episodes like `1 3 5`. Use `0` for the first and `-1` for the last episode. |
| `-q, --quality QUALITY` | Request `best`, `worst`, or a resolution such as `720p`. |
| `--dub` | Select dubbed audio when available. |
| `-v, --vlc` / `-s, --syncplay` | Select VLC or Syncplay. |
| `--rofi` / `--dmenu` | Use an external selection menu. |
| `--skip` | Enable intro skipping where supported. |
| `--no-detach` | Keep the player attached to the terminal. |
| `--exit-after-play` | Return the player's exit code after playback. |
| `-N, --nextep-countdown` | Show the next-episode countdown. |
| `-D, --delete` / `-l, --logview` | Delete watch history or view logs. |
| `-U, --update` | Update the Cargo installation from a Git branch. |

## Runtime dependencies

| Tool | When it is needed |
| --- | --- |
| MPV, VLC, IINA, Syncplay, or a supported platform player | Required for playback; install a player accessible to ani-cli. |
| FFmpeg | Downloads and muxing separate audio/video streams. |
| yt-dlp | Optional; preferred for supported single-stream downloads. |
| fzf, rofi, or dmenu | Optional selection backends. |
| ani-skip | Optional intro skipping for supported MPV/Syncplay setups. |

Install players and command-line tools with your operating system's package manager. On Windows, ensure `mpv.exe` or `vlc.exe` is available through `PATH`, or set `ANI_CLI_PLAYER` to the executable path. WSL may need extra setup to launch a Windows player. Android/Termux support depends on the player app and intent integration; this project does not ship a native Android binary.

## Configuration

| Variable | Purpose |
| --- | --- |
| `ANI_CLI_DEFAULT_SOURCE` | Default action: `search`, `history`, or `nextep`. |
| `ANI_CLI_MODE` | Default audio mode: `sub` or `dub`. |
| `ANI_CLI_QUALITY` | Default stream quality. |
| `ANI_CLI_EXACT=1` | Require a normalized exact title match. |
| `ANI_CLI_MENU` | Default menu backend: `fzf`, `rofi`, or `dmenu`. |
| `ANI_CLI_MENU_FLAGS` | Extra arguments for the menu program. |
| `ANI_CLI_PLAYER` | Player executable or supported player name. |
| `ANI_CLI_PLAYER_FLAGS` | Extra player/downloader arguments. |
| `ANI_CLI_DOWNLOAD_DIR` | Download destination directory. |
| `ANI_CLI_HIST_DIR` | Directory for the compatible history file. |
| `ANI_CLI_NO_DETACH=1` | Keep playback attached to the terminal. |
| `ANI_CLI_EXIT_AFTER_PLAY=1` | Return the player exit status. |
| `ANI_CLI_SKIP_INTRO=1` | Enable supported intro skipping. |
| `ANI_CLI_LOG=0` | Disable system logger records. |

## Caveats

- HiAnime markup, stream hosts, and anti-bot responses can change without notice. CI does not guarantee live-provider availability.
- Prebuilt release binaries are not published for the Rust port yet. See the [migration notes](docs/rust-migration.md) for platform caveats.
- The client does not host or own the media it locates. You are responsible for complying with applicable laws and service terms; read the [disclaimer](docs/disclaimer.md).

## Documentation and support

- [Step-by-step tutorial](docs/tutorial.md)
- [Development guide](docs/development.md)
- [Contributing guide](docs/contributing.md)
- [Rust migration notes](docs/rust-migration.md)
- [Roadmap](docs/roadmap.md)
- [Matrix community](docs/matrix.md)
- [Open an issue](https://github.com/gabrielfranicevich/anime/issues)

## License

ani-cli is distributed under **GPL-3.0-only**. See [LICENSE](LICENSE).
