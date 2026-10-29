# watch tutorial

This tutorial covers installation, searching, playback, downloads, and watch history. `main` is the stable installation target; `android-support` contains ongoing Android work.

## 1. Install watch

Install the published package with Cargo:

~~~sh
cargo install watch
watch --version
~~~

If the package is not available from crates.io, install the main branch from GitHub:

~~~sh
cargo install --git https://github.com/gabrielfranicevich/watch --branch main
~~~

If `watch` is not found, make sure Cargo's binary directory (`~/.cargo/bin`, or the equivalent on your platform) is on your `PATH`. On Linux, `watch` may conflict with the system process-monitoring utility; place `~/.cargo/bin` before system directories or run `~/.cargo/bin/watch` explicitly.

To build from a checkout instead:

~~~sh
git clone --branch main https://github.com/gabrielfranicevich/watch.git
cd watch
cargo build --release
./target/release/watch --version
~~~

Prebuilt binaries and updated packages for third-party package managers are not published yet.

## 2. Install playback tools

Install a supported player separately. MPV is the usual default; VLC, Syncplay, and IINA are also supported where applicable. On Windows, install MPV or VLC and make sure its executable is available through `PATH`. watch must be able to find the player from the same terminal or environment that launches it. If you use a custom location, set `ANI_CLI_PLAYER` to the executable path.

FFmpeg is needed for downloads and for muxing separate video and audio tracks. yt-dlp is optional and preferred when a resolved stream is suitable. fzf, rofi, and dmenu are optional selection menus.

On Windows, install tools in the same environment from which you launch watch. WSL may need extra setup to invoke a Windows player.

### Android / Termux

See the dedicated [Android / Termux installation guide](android.md) for installing Watch, setting up MPV-Android's shared configuration, and testing playback.

## 3. Search and play

Run a title query:

~~~sh
watch "Frieren"
~~~

Filter the catalog by language with `--language`. Values are case-insensitive; English accepts `english`, `en`, `ingles`, or `inglés`, and Spanish accepts `spanish`, `es`, `espanol`, or `español`:

~~~sh
watch --language english "Dragon Ball"
watch --language ESPAÑOL "Dragon Ball"
~~~

This filters catalog language, not audio. Use `--dub` to request dubbed audio.

Select a matching title, then choose an episode. The client resolves the stream and passes the URL, referer, and available subtitle/audio options to the configured player.

### Choosing entries and going back

If there is only one anime result or one episode, `watch` tells you it is the only available choice and selects it automatically.

The built-in numbered menu shows up to 15 results at a time. In a directly attached interactive terminal, use Left/Right to change pages; the menu redraws those rows in place. If a launcher does not expose console input, the line-based fallback accepts `n` and `p` and asks for a number on the current page. Numbers refer to the full result list. If `fzf` is installed, it can filter and scroll all results.

Press `Backspace` with an empty selection to go back: from episode selection to anime results, or from the anime results to a fresh search prompt. When a single anime result was selected automatically, going back from its episode list returns to search rather than choosing the same title again. Use `q` or `Esc` to cancel; cancellation exits successfully without an error. With `fzf`, Backspace is reserved for going back rather than deleting characters from the filter.

Request a resolution with `--quality`:

~~~sh
watch --quality 720p "Frieren"
watch --quality best "Frieren"
~~~

Quality options depend on the selected source and episode. Episode lists combine sources for a title. For each selected episode, Watch tries the providers that list it and chooses the resolved stream that best matches `--quality`. The default `best` selects the highest available quality.

## 4. Download episodes

Use `--download` with a range:

~~~sh
watch --download --episode 1-3 "Frieren"
~~~

You can also choose individual episode numbers:

~~~sh
watch --download --episode "1 3 5" "Frieren"
~~~

`--range` is an alias for `--episode`. `0` means the first available episode and `-1` means the last. Files are saved in the current directory unless `ANI_CLI_DOWNLOAD_DIR` is set.

## 5. Continue from history

To continue from the last recorded episode:

~~~sh
watch --continue
~~~

The history file is stored in a platform-specific data directory. Set `ANI_CLI_HIST_DIR` to use a compatible history location. Use `watch --delete` only when you intend to delete that history.

## 6. Choose a player or menu

~~~sh
# Use VLC
watch --vlc "Frieren"

# Use Syncplay
watch --syncplay "Frieren"

# Use an external menu
watch --rofi "Frieren"
watch --dmenu "Frieren"
~~~

Set `ANI_CLI_PLAYER` to a supported player name or executable path, and `ANI_CLI_MENU` to `fzf`, `rofi`, or `dmenu`. `ANI_CLI_PLAYER_FLAGS` and `ANI_CLI_MENU_FLAGS` pass additional arguments to the corresponding tools.

## 7. Play inside the terminal

On supported desktop terminals, use `--inline` (or its alias, `--here`) to render the video in the terminal instead of opening a regular video window. These flags are not supported on Android/Termux; omit them to use external playback:

~~~sh
watch --inline "Frieren"
watch "Frieren" --here
~~~

Inline playback is experimental and depends on both MPV and the terminal. MPV must include a terminal output driver, and the terminal must implement that driver's protocol. A driver listed by `mpv --vo=help` does not prove that the terminal supports its protocol. Kitty and Sixel are different graphics protocols; `tct` uses ANSI color cells and is not equivalent to either one. Windows Terminal support can vary by protocol and setup.

If you want to try it, run:

~~~sh
# Choose a backend explicitly
ANI_CLI_TERMINAL_BACKEND=kitty watch --inline "Frieren"
ANI_CLI_TERMINAL_BACKEND=sixel watch --inline "Frieren"
ANI_CLI_TERMINAL_BACKEND=tct watch --inline "Frieren"
~~~

The setting selects a driver; it cannot add protocol support to the terminal. If MPV plays audio but no frames appear, use ordinary windowed playback and verify the protocol with a controlled local video before trying another backend. Inline mode is incompatible with `--download`, `--vlc`, and `--syncplay`.

## 8. Useful commands

| Command | Result |
| --- | --- |
| `watch --help` | Show available options. |
| `watch --version` | Show the installed version. |
| `watch --continue` | Resume from history. |
| `watch --nextep-countdown "Frieren"` | Show the next-episode countdown; depends on AnimeSchedule data. |
| `watch --logview` | View available application logs. |
| `watch --update` | Update a Cargo installation; requires Cargo and network access. |

## Troubleshooting

**Search fails or returns a verification page.** The upstream provider may be applying anti-bot checks or may have changed its page structure. Retry later; this is not necessarily an watch configuration problem.

**Playback fails with `program not found`.** watch resolves the stream but delegates playback to an external player. Install MPV or VLC and make sure the player executable is available through `PATH` in the environment where watch runs. Inline mode on Windows also needs MPV's console launcher, commonly distributed beside `mpv.exe`; this does not guarantee that the terminal supports its graphics protocol. To use a custom location, set `ANI_CLI_PLAYER` to the executable path. You can also try `watch --vlc "Frieren"` when VLC is installed.

**Playback does not start for another reason.** Confirm that the selected player supports the stream and is accessible from your terminal, then inspect the error output.

**Inline terminal playback is blank or garbled.** MPV listing a driver does not establish that the terminal supports its protocol. Try regular windowed playback while checking the terminal emulator's protocol support. Kitty and Sixel are separate graphics protocols; `tct` draws colored terminal cells. Do not expect changing the selected driver alone to make an unsupported terminal render frames.

**Download fails.** Install FFmpeg and, if available, yt-dlp. Some stream sources may be unavailable or unsuitable for download even when playback works.

**The command is not found after Cargo installation.** Add Cargo's binary directory to your `PATH`, then open a new terminal.

For platform and migration notes, see [Rust migration notes](rust-migration.md). To report a reproducible problem, include the Watch version, operating system, terminal emulator when relevant, command, expected behavior, and error output in a [GitHub issue](https://github.com/gabrielfranicevich/watch/issues).
