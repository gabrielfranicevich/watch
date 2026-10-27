# watch

Search anime, select episodes, and stream or download them from your terminal.

[![Rust CI](https://github.com/gabrielfranicevich/watch/actions/workflows/rust.yml/badge.svg?branch=develop)](https://github.com/gabrielfranicevich/watch/actions/workflows/rust.yml)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)
![Rust 1.85+](https://img.shields.io/badge/Rust-1.85%2B-orange)

watch searches English and Spanish anime providers, groups duplicate titles by language, and hands playable media or downloads to tools installed on your machine. It does not host video files.

> **Project status:** the Rust implementation is developed on `develop`. Prebuilt binaries and updated third-party packages are not published yet; some package managers may still provide the historical shell version from `main`.

<details>
<summary>Contents</summary>

- [Why watch?](#why-watch)
- [Quick start](#quick-start)
- [Examples](#examples)
- [Interactive selection](#interactive-selection)
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
cargo install --git https://github.com/gabrielfranicevich/watch --branch develop
watch "Frieren"
~~~

Choose a result and an episode from the interactive menus. Playback requires a supported media player, such as MPV or VLC, installed and available on `PATH`. On Windows, install the player in the same environment where you run watch. See the [tutorial](docs/tutorial.md) for setup and troubleshooting.

On Linux, `watch` may already refer to the system process-monitoring utility. If the wrong command runs, put Cargo’s binary directory (`~/.cargo/bin`) before system directories in `PATH`, or invoke `~/.cargo/bin/watch` explicitly. The Rust package and executable are both named `watch`; legacy `ANI_CLI_*` environment variable names remain unchanged.

## Examples

~~~sh
# Search and play interactively
watch "Frieren"

# Prefer 720p when the source offers it
watch --quality 720p "Frieren"

# Download episodes 1 through 3
watch --download --episode 1-3 "Frieren"

# Download individual episodes
watch --download --episode "1 3 5" "Frieren"

# Render video inside a supported terminal
watch --inline "Frieren"

# Continue from watch history
watch --continue

# Show the next-episode countdown
watch --nextep-countdown "Frieren"
~~~

Run `watch --help` for the complete option list.

## Interactive selection

- A single anime result is selected automatically with a message saying it is the only result. The same rule applies when an anime has only one available episode.
- The built-in numbered menu displays up to 15 results per page. Press Left or Right to move between pages; item numbers remain global across pages. Page-navigation hints appear only when more than one page is available.
- Press `Backspace` with no number entered to go back. From episode selection it returns to the anime results; from the anime results it starts a fresh search. If the only anime result was auto-selected, Backspace from its episode list returns to the search prompt instead of reopening that same result.
- Press `q` or `Esc` to cancel the current selection.
- When `fzf` is available, it can filter and scroll the complete list. Backspace is reserved for navigating to the previous step rather than deleting text from the `fzf` filter.

## Why watch?

watch is for terminal users who want to search for anime, choose episodes, resume from watch history, and download media through one command-line workflow. It coordinates these steps while leaving playback and media processing to tools installed on your system.

## What it supports

- Parallel title search across HiAnime, Gogoanime, AniWatch, 9anime, AnimeKai, MonosChinos, TioAnime, Animeblix, AnimeFenix, and JKAnime.
- Duplicate titles are grouped by normalized title and language; when a title appears in both languages it is listed as separate `(en)` and `(es)` choices, and you can choose the provider after selecting a title.
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
| `-S, --select-nth N` | Select an entry by its one-based index. |
| `--dub` | Select dubbed audio when available. |
| `-v, --vlc` / `-s, --syncplay` | Select VLC or Syncplay. |
| `--rofi` / `--dmenu` | Use an external selection menu. |
| `--skip` | Enable intro skipping where supported. |
| `--no-detach` | Keep the player attached to the terminal. |
| `--inline`, `--here` | Render video in the terminal using an available graphics protocol. On Windows, automatic mode requires MPV's Sixel output; `tct` is only used when explicitly selected. |
| `--exit-after-play` | Return the player's exit code after playback. |
| `-N, --nextep-countdown` | Show the next-episode countdown. |
| `-D, --delete` / `-l, --logview` | Delete watch history or view logs. |
| `-U, --update` | Update the Cargo installation from a Git branch. |

## Runtime dependencies

| Tool | When it is needed |
| --- | --- |
| MPV, VLC, IINA, Syncplay, or a supported platform player | Required for playback; install a player accessible to watch. `--inline` requires MPV with a terminal video output driver. |
| FFmpeg | Downloads and muxing separate audio/video streams. |
| yt-dlp | Optional; preferred for supported single-stream downloads. |
| fzf, rofi, or dmenu | Optional selection backends. |
| ani-skip | Optional intro skipping for supported MPV/Syncplay setups. |

Install players and command-line tools with your operating system's package manager. On Windows, ensure `mpv.com` and `mpv.exe` (for inline playback) or `vlc.exe` are available through `PATH`, or set `ANI_CLI_PLAYER` to the executable path. WSL may need extra setup to launch a Windows player. For terminal video output and backend overrides, see the [tutorial](docs/tutorial.md#7-play-inside-the-terminal). Android/Termux support depends on the player app and intent integration; this project does not ship a native Android binary.

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
| `ANI_CLI_TERMINAL_BACKEND` | Inline output backend: `auto`, `kitty`, `sixel`, or `tct`. |
| `ANI_CLI_DOWNLOAD_DIR` | Download destination directory. |
| `ANI_CLI_HIST_DIR` | Directory for the compatible history file. |
| `ANI_CLI_NO_DETACH=1` | Keep playback attached to the terminal. |
| `ANI_CLI_EXIT_AFTER_PLAY=1` | Return the player exit status. |
| `ANI_CLI_SKIP_INTRO=1` | Enable supported intro skipping. |
| `ANI_CLI_LOG=0` | Disable system logger records. |

## Caveats

- Provider markup, stream hosts, search routes, and anti-bot responses can change without notice. Non-HiAnime providers use shared HTML and direct HLS/MP4 extraction; sites with encrypted or custom embedded players may require provider-specific resolver updates. CI does not guarantee live-provider availability.
- Prebuilt release binaries are not published for the Rust port yet. See the [migration notes](docs/rust-migration.md) for platform caveats.
- The client does not host or own the media it locates. You are responsible for complying with applicable laws and service terms; read the [disclaimer](docs/disclaimer.md).

## Documentation and support

- [Step-by-step tutorial](docs/tutorial.md)
- [Development guide](docs/development.md)
- [Contributing guide](docs/contributing.md)
- [Rust migration notes](docs/rust-migration.md)
- [Roadmap](docs/roadmap.md)
- [Upstream Matrix community (original maintainers)](docs/matrix.md)
- [Open an issue](https://github.com/gabrielfranicevich/watch/issues)

## License

watch is distributed under **GPL-3.0-only**. See [LICENSE](LICENSE).
