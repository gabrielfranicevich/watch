> **Rust migration (`develop`):** the shell version on `main` remains the reference implementation while compatibility is validated. To build locally, check out `develop` and run `cargo run --release -- "anime name"`. To install the experimental Rust binary with Cargo, run `cargo install --git https://github.com/gabrielfranicevich/anime --branch develop`. This branch is not yet a claim of full package-manager parity; see [migration notes](docs/rust-migration.md).

<p align=center>
<br>
<a href="http://makeapullrequest.com"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg"></a>
<a href="#Linux"><img src="https://img.shields.io/badge/os-linux-brightgreen">
<a href="#MacOS"><img src="https://img.shields.io/badge/os-mac-brightgreen">
<a href="#Windows"><img src="https://img.shields.io/badge/os-windows-yellowgreen">
<a href="#Android"><img src="https://img.shields.io/badge/os-android-yellow">
<a href="#Steam-deck"><img src="https://img.shields.io/badge/os-steamdeck-yellow">
<a href="#iOS"><img src="https://img.shields.io/badge/os-ios-red">
<br>
<a href="https://stats.coolans.dev/endpoints/website_ani-cli"><img src="https://stats.coolans.dev/api/v1/endpoints/website_ani-cli/health/badge.svg"></a>
<a href="https://stats.coolans.dev/endpoints/website_ani-cli"><img src="https://stats.coolans.dev/api/v1/endpoints/website_ani-cli/uptimes/7d/badge.svg"></a>
<br>
<p align=center>
<a href="https://discord.gg/aqu7GpqVmR"><img src="https://invidget.switchblade.xyz/aqu7GpqVmR"></a>
<a href="matrix.md"><img src="/.assets/matrix-logo.svg" height=110></a>
<br>
<a href="https://discord.gg/aqu7GpqVmR">Discord</a>
<a href="https://github.com/pystardust/ani-cli/blob/master/matrix.md">Matrix</a>
</p>
<a href="https://github.com/port19x"><img src="https://img.shields.io/badge/lead-port19x-lightblue"></a>
<a href="https://github.com/CoolnsX"><img src="https://img.shields.io/badge/maintainer-CoolnsX-blue"></a>
<a href="https://github.com/justchokingaround"><img src="https://img.shields.io/badge/maintainer-justchokingaround-blue"></a>
<a href="https://github.com/Derisis13"><img src="https://img.shields.io/badge/maintainer-Derisis13-blue"></a>
<a href="https://github.com/71zenith"><img src="https://img.shields.io/badge/maintainer-71zenith-blue"></a>
<a href="https://github.com/vorlie"><img src="https://img.shields.io/badge/maintainer-vorlie-blue"></a>

</p>

<h3 align="center">
A cli to browse and watch anime (alone AND with friends). This tool scrapes the site <a href="https://hianime.at/">hianime.</a>
</h3>

<h1 align="center">
	Showcase
</h1>

[ani-cli-demo.webm](https://user-images.githubusercontent.com/44473782/224679247-0856e652-f187-4865-bbcf-5a8e5cf830da.webm)

## Table of Contents

- [Install the Rust CLI](#install)
- [Dependencies](#dependencies)
- [Ani-Skip](#ani-skip)
- [FAQ](#faq)
- [Related projects](#homies)
- [Contribution Guidelines](./CONTRIBUTING.md)
- [Disclaimer](./disclaimer.md)

## Install

This branch contains the Rust implementation. The `main` branch remains the reference for the historical shell implementation while the migration continues to be validated. Prebuilt binary releases are not published yet.

### Install with Cargo

Requires a stable Rust toolchain (Rust 1.85 or newer) and Git:

```sh
cargo install --git https://github.com/gabrielfranicevich/anime --branch develop
ani-cli --help
```

To update a Cargo installation, run `ani-cli --update` or reinstall from the same branch:

```sh
cargo install --force --git https://github.com/gabrielfranicevich/anime --branch develop
```

### Build from source

```sh
git clone --branch develop https://github.com/gabrielfranicevich/anime.git
cd anime
cargo build --release
./target/release/ani-cli --help
./target/release/ani-cli "Frieren"
```

Download episodes with `--download`, for example:

```sh
ani-cli --download --episode 1-3 "Frieren"
```

### Runtime support

The Rust crate is checked in CI on Linux, macOS and Windows. CI verifies compilation, Clippy, unit/regression tests, CLI help/version and formatting; it does not guarantee that every external media player or live provider endpoint works on every OS.

Playback requires a supported external player. MPV is the default where available; VLC is selectable with `--vlc`, Syncplay with `--syncplay`, and IINA can be selected on macOS. Install the player separately and ensure it is on `PATH`, except for platform integrations that launch the app explicitly.

For Windows, install Rust and a supported media player in the Windows environment. WSL needs an intentional player setup because a Linux process may not be able to launch the Windows player with Linux-style arguments. Android/Termux support depends on the player app and Android intent integration; it is not packaged as a native Android binary.

The supported feature set includes HiAnime search, episode selection, HLS stream resolution, subtitles, playback, downloads, quality selection and compatible watch history. Provider markup, anti-bot challenges and stream hosts are external dependencies and may change.

### Legacy package-manager caveat

Packages in third-party distribution repositories (APT, DNF, AUR, Scoop, Termux and similar) may still package the historical shell release. They are not guaranteed to install the Rust implementation from this branch. Until a release and packages are published for the Rust version, use the Cargo commands above.

## Uninstall

For a Cargo installation:

```sh
cargo uninstall ani-cli
```

If you built from source, remove the resulting executable from the directory into which you installed it. This does not remove the watch-history file.

## Dependencies

- Rust and Cargo for installation/building.
- A supported external media player (MPV, VLC, IINA or Syncplay, depending on the platform and selected options).
- FFmpeg for downloads and for muxing separate audio/video streams.
- yt-dlp is optional and preferred for single-stream downloads when installed.
- fzf, rofi or dmenu are optional selection backends.
- ani-skip is optional and only works with supported MPV/Syncplay setups.

### Ani-Skip

Ani-skip is a script to automatically skip anime opening sequences, making it easier to watch your favorite shows without having to manually skip the intros each time (from the original [README](https://github.com/synacktraa/ani-skip/tree/master#a-script-to-automatically-skip-anime-opening-sequences-making-it-easier-to-watch-your-favorite-shows-without-having-to-manually-skip-the-intros-each-time)).

For install instructions visit [ani-skip](https://github.com/synacktraa/ani-skip).

Ani-skip uses the external lua script function of mpv and as such – for now – only works with mpv.

**Warning:** For now, ani-skip does **not** seem to work under Windows.

## FAQ

- **Can I change or disable subtitles?** Subtitles are passed to the external player. Use the player's controls to toggle the English track; other subtitle languages are not currently offered.
- **Can I watch dubbed audio?** Use `--dub` when a dub stream is available.
- **Can I change resolution?** Use `--quality 720p`, or select `best`/`worst`.
- **How do I download episodes?** Use `--download`, optionally with `--episode 1-3`.
- **Where are downloads saved?** Set `ANI_CLI_DOWNLOAD_DIR` to choose the output directory.
- **Can I use VLC or another player?** Use `--vlc`, `--syncplay`, or set `ANI_CLI_PLAYER` to a supported player.
- **How do I continue watching?** Use `--continue` to resume from compatible watch history.
- **How do I update?** Use `--update` for the Cargo-based updater, or reinstall with `cargo install --force --git https://github.com/gabrielfranicevich/anime --branch develop`. Updating requires Cargo and network access.

## Homies

* [ani-cli-rs](https://github.com/vorlie/ani-cli-rs): A cross-platform Rust port of ani-cli with two independent Anikoto catalogs and native MegaPlay/KotoCDN playback. (Rust)
* [jerry](https://github.com/justchokingaround/jerry): stream anime with anilist tracking and syncing, with discord presence (Shell)
* [anipy-cli](https://github.com/sdaqo/anipy-cli): ani-cli rewritten in python (Python)
* [mov-cli](https://github.com/mov-cli/mov-cli): Watch everything from your terminal. (Python)
* [doccli](https://github.com/TowarzyszFatCat/doccli): [LINUX / WINDOWS] A CLI to watch anime with ENGLISH sub/dub or POLISH subtitles options, AniList integration, Discord RPC, and native Windows installer (Python)
* [GoAnime](https://github.com/alvarorichard/GoAnime): A TUI tool to browse, play, and download anime in Portuguese and English, with Discord RPC, AniList integration, and intro skipping. (Go)
* [Curd](https://github.com/Wraient/curd): A CLI tool to watch anime with Anilist, Discord RPC, Skip Intro/Outro/Filler/Recap (Go)
* [ani-skip](https://github.com/synacktraa/ani-skip): Automatically skip opening and ending sequences for IINA on MacOS (Typescript, official IINA plugin API)
