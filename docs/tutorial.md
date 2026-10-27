# watch tutorial

This tutorial covers installation, searching, playback, downloads, and watch history for the Rust implementation on `develop`.

## 1. Install watch

Install stable Rust 1.85 or newer and Git, then run:

~~~sh
cargo install --git https://github.com/gabrielfranicevich/watch --branch develop
watch --version
~~~

If `watch` is not found, make sure Cargo's binary directory (`~/.cargo/bin`, or the equivalent on your platform) is on your `PATH`. On Linux, `watch` may conflict with the system process-monitoring utility; place `~/.cargo/bin` before system directories or run `~/.cargo/bin/watch` explicitly.

To build from a checkout instead:

~~~sh
git clone --branch develop https://github.com/gabrielfranicevich/watch.git
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

The Rust CLI can be built directly in Termux on supported ARM64 devices. Install the build tools and clone the Android-support branch:

~~~sh
pkg update
pkg install rust git clang
git clone --branch android-support https://github.com/gabrielfranicevich/watch.git
cd watch
cargo build --release
./target/release/watch --version
~~~

On Android, external playback uses an Android `VIEW` intent by default, allowing Android to offer a compatible installed app. If `ANI_CLI_PLAYER` names MPV or VLC, watch uses that app's explicit activity integration instead. Install a media player that can open the provider's stream format. The player app may not support referer-dependent HLS streams, external audio, or subtitles.

The `--inline` option attempts MPV terminal output when a compatible interactive terminal and output driver are available. If inline setup or launching the inline player fails, watch switches to external playback. Terminal graphics protocols are not guaranteed to work in Termux, so external playback is the expected fallback. Android device playback has not yet been validated by this documentation change; report device model, Android version, Termux version, player app, command, and error output when troubleshooting.

## 3. Search and play

Run a title query:

~~~sh
watch "Frieren"
~~~

Select a matching title, then choose an episode. The client resolves the stream and passes the URL, referer, and available subtitle/audio options to the configured player.

### Choosing entries and going back

If there is only one anime result or one episode, `watch` tells you it is the only available choice and selects it automatically.

The built-in numbered menu shows up to 15 results at a time. Press `n` or `p` when no selection digits have been entered to change pages. Numbers refer to the full result list, not just the visible page. If `fzf` is installed, it can filter and scroll all results.

Press `Backspace` with an empty selection to go back: from episode selection to anime results, or from the anime results to a fresh search prompt. When a single anime result was selected automatically, going back from its episode list returns to search rather than choosing the same title again. Use `q` or `Esc` to cancel the current selection. With `fzf`, Backspace is reserved for going back rather than deleting characters from the filter.

Request a resolution with `--quality`:

~~~sh
watch --quality 720p "Frieren"
watch --quality best "Frieren"
~~~

Quality options depend on the selected source and episode.

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

Use `--inline` (or its alias, `--here`) to render the video in the terminal instead of opening a regular video window:

~~~sh
watch --inline "Frieren"
watch "Frieren" --here
~~~

This mode requires MPV with a terminal video output driver and an interactive terminal on both stdin and stdout. Before playback, watch runs `mpv --vo=help` using the selected MPV executable, chooses a compiled-in terminal driver, and disables the normal video window. On Windows, watch launches MPV through `mpv.com`, the console launcher, so terminal video output has a console handle; keep `mpv.com` and `mpv.exe` together on `PATH`. Kitty-protocol terminals prefer Kitty graphics; Windows defaults to Sixel because MPV's `tct` driver only works on Windows in ANSI-compatible terminals such as mintty.

Terminal detection is heuristic and cannot guarantee that a particular terminal actually renders a protocol. On non-Windows systems, watch can fall back to `tct` when a graphics driver is missing. On Windows, it deliberately does not fall back to `tct` automatically: if the installed MPV lacks Sixel output, it reports an error rather than starting audio-only playback. You can explicitly request `tct` only when using an ANSI-compatible terminal:

~~~sh
# Choose a backend explicitly
ANI_CLI_TERMINAL_BACKEND=kitty watch --inline "Frieren"
ANI_CLI_TERMINAL_BACKEND=sixel watch --inline "Frieren"
ANI_CLI_TERMINAL_BACKEND=tct watch --inline "Frieren"
~~~

Use `auto` or unset the variable to restore detection. To see the output drivers in your MPV build, run `mpv --vo=help`. Inline mode is incompatible with `--download`, `--vlc`, and `--syncplay`; use a regular player for those workflows.

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

**Playback fails with `program not found`.** watch resolves the stream but delegates playback to an external player. Install MPV or VLC and make sure `mpv.com`/`mpv.exe` or `vlc.exe` is available through `PATH` in the environment where watch runs. On Windows inline playback uses the console launcher `mpv.com`, which is normally distributed beside `mpv.exe`. To use a custom location, set `ANI_CLI_PLAYER` to the executable path. You can also try `watch --vlc "Frieren"` when VLC is installed.

**Playback does not start for another reason.** Confirm that the selected player supports the stream and is accessible from your terminal, then inspect the error output.

**Inline terminal playback is blank or garbled.** Run `mpv --vo=help` and check that MPV lists `kitty`, `sixel`, or `tct`. watch checks this list before playback and reports an error if no terminal output driver is available. You can override `ANI_CLI_TERMINAL_BACKEND` with `kitty`, `sixel`, or `tct`. Kitty and Sixel require graphics-protocol support from the terminal emulator; `tct` draws frames using colored terminal cells.

**Download fails.** Install FFmpeg and, if available, yt-dlp. Some stream sources may be unavailable or unsuitable for download even when playback works.

**The command is not found after Cargo installation.** Add Cargo's binary directory to your `PATH`, then open a new terminal.

For platform caveats, see the [Rust migration notes](rust-migration.md). To report a reproducible problem, include the watch version, operating system, command, expected behavior, and error output in a [GitHub issue](https://github.com/gabrielfranicevich/watch/issues).
