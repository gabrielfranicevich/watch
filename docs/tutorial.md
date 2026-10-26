# ani-cli tutorial

This tutorial covers installation, searching, playback, downloads, and watch history for the Rust implementation on `develop`.

## 1. Install ani-cli

Install stable Rust 1.85 or newer and Git, then run:

~~~sh
cargo install --git https://github.com/gabrielfranicevich/anime --branch develop
ani-cli --version
~~~

If `ani-cli` is not found, make sure Cargo's binary directory (`~/.cargo/bin`, or the equivalent on your platform) is on your `PATH`.

To build from a checkout instead:

~~~sh
git clone --branch develop https://github.com/gabrielfranicevich/anime.git
cd anime
cargo build --release
./target/release/ani-cli --version
~~~

Prebuilt binaries and updated packages for third-party package managers are not published yet.

## 2. Install playback tools

Install a supported player separately. MPV is the usual default; VLC, Syncplay, and IINA are also supported where applicable. On Windows, install MPV or VLC and make sure its executable is available through `PATH`. ani-cli must be able to find the player from the same terminal or environment that launches it. If you use a custom location, set `ANI_CLI_PLAYER` to the executable path.

FFmpeg is needed for downloads and for muxing separate video and audio tracks. yt-dlp is optional and preferred when a resolved stream is suitable. fzf, rofi, and dmenu are optional selection menus.

On Windows, install tools in the same environment from which you launch ani-cli. WSL may need extra setup to invoke a Windows player. On Android/Termux, launching a player depends on the installed app and Android intent integration.

## 3. Search and play

Run a title query:

~~~sh
ani-cli "Frieren"
~~~

Select a matching title, then choose an episode. The client resolves the stream and passes the URL, referer, and available subtitle/audio options to the configured player.

Request a resolution with `--quality`:

~~~sh
ani-cli --quality 720p "Frieren"
ani-cli --quality best "Frieren"
~~~

Quality options depend on the selected source and episode.

## 4. Download episodes

Use `--download` with a range:

~~~sh
ani-cli --download --episode 1-3 "Frieren"
~~~

You can also choose individual episode numbers:

~~~sh
ani-cli --download --episode "1 3 5" "Frieren"
~~~

`--range` is an alias for `--episode`. `0` means the first available episode and `-1` means the last. Files are saved in the current directory unless `ANI_CLI_DOWNLOAD_DIR` is set.

## 5. Continue from history

To continue from the last recorded episode:

~~~sh
ani-cli --continue
~~~

The history file is stored in a platform-specific data directory. Set `ANI_CLI_HIST_DIR` to use a compatible history location. Use `ani-cli --delete` only when you intend to delete that history.

## 6. Choose a player or menu

~~~sh
# Use VLC
ani-cli --vlc "Frieren"

# Use Syncplay
ani-cli --syncplay "Frieren"

# Use an external menu
ani-cli --rofi "Frieren"
ani-cli --dmenu "Frieren"
~~~

Set `ANI_CLI_PLAYER` to a supported player name or executable path, and `ANI_CLI_MENU` to `fzf`, `rofi`, or `dmenu`. `ANI_CLI_PLAYER_FLAGS` and `ANI_CLI_MENU_FLAGS` pass additional arguments to the corresponding tools.

## 7. Useful commands

| Command | Result |
| --- | --- |
| `ani-cli --help` | Show available options. |
| `ani-cli --version` | Show the installed version. |
| `ani-cli --continue` | Resume from history. |
| `ani-cli --nextep-countdown "Frieren"` | Show the next-episode countdown; depends on AnimeSchedule data. |
| `ani-cli --logview` | View available application logs. |
| `ani-cli --update` | Update a Cargo installation; requires Cargo and network access. |

## Troubleshooting

**Search fails or returns a verification page.** The upstream provider may be applying anti-bot checks or may have changed its page structure. Retry later; this is not necessarily an ani-cli configuration problem.

**Playback fails with `program not found`.** ani-cli resolves the stream but delegates playback to an external player. Install MPV or VLC and make sure `mpv.exe` or `vlc.exe` is available through `PATH` in the environment where ani-cli runs. To use a custom location, set `ANI_CLI_PLAYER` to the executable path. You can also try `ani-cli --vlc "Frieren"` when VLC is installed.

**Playback does not start for another reason.** Confirm that the selected player supports the stream and is accessible from your terminal, then inspect the error output.

**Download fails.** Install FFmpeg and, if available, yt-dlp. Some stream sources may be unavailable or unsuitable for download even when playback works.

**The command is not found after Cargo installation.** Add Cargo's binary directory to your `PATH`, then open a new terminal.

For platform caveats, see the [Rust migration notes](rust-migration.md). To report a reproducible problem, include the ani-cli version, operating system, command, expected behavior, and error output in a [GitHub issue](https://github.com/gabrielfranicevich/anime/issues).
