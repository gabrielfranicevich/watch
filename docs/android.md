# Android / Termux installation guide

This guide covers the **watch CLI** on Android through [Termux](https://termux.dev/). It does not cover a native Android application: the repository does not currently contain an APK or Kotlin/Jetpack Compose app.

## Status and limitations

- The Rust CLI can be built for Android ARM64; GitHub Actions currently checks the `aarch64-linux-android` target.
- Runtime playback has **not** been verified on a physical Android device.
- The current Android player integration launches the MPV Android activity by default. The `--vlc` option selects the VLC Android activity.
- The CLI does **not yet** implement generic Android media-handler fallback or reliable inline-to-external fallback. The playback policy module has unit tests, but the policy is not integrated into the current playback flow.
- A stream may require a referer, separate audio, or subtitles that a player activity cannot receive correctly. Provider availability and player compatibility vary.

## 1. Install Termux

Install Termux from a trusted, current distribution source. Do not mix Termux and its add-ons from different signing sources. Open Termux and update its packages:

~~~sh
pkg update && pkg upgrade
~~~

## 2. Install build tools

Install Rust, Git, and a C toolchain:

~~~sh
pkg install rust git clang
~~~

If Cargo reports a missing native library or linker, install the package that provides the missing dependency and retry. Keep the complete error output when reporting build failures.

## 3. Download and build watch

Clone the development branch that contains the Android work:

~~~sh
git clone --branch android-support https://github.com/gabrielfranicevich/watch.git
cd watch
cargo build --release
~~~

Verify that the binary starts:

~~~sh
./target/release/watch --version
./target/release/watch --help
~~~

Run it from the checkout with:

~~~sh
./target/release/watch "Frieren"
~~~

The repository's Rust package and executable are named `watch`. If you later install the binary into Cargo's bin directory, make sure that directory is on `PATH`. On Linux-like systems, the name `watch` can also refer to a system process-monitoring utility; use an explicit path if the wrong executable runs.

## 4. Install a compatible Android player

The current Android integration expects an Android player app, not just a command-line package inside Termux.

- **MPV:** install a compatible MPV Android app. This is the default player path.
- **VLC:** install VLC for Android and select it with `--vlc`.

Try a provider stream in the player app itself first. Some sites use HLS playlists, HTTP referer checks, or separate audio and subtitle URLs; a player may open the URL but still fail to play the stream correctly.

The current MPV integration targets the activity `is.xyz.mpv/.MPVActivity`; the VLC integration targets `org.videolan.vlc/org.videolan.vlc.gui.video.VideoPlayerActivity`. If your installed app uses a different package/activity, the current explicit integration may not work.

## 5. Start playback

Search interactively:

~~~sh
./target/release/watch "Frieren"
~~~

Select an anime and episode. To request VLC:

~~~sh
./target/release/watch --vlc "Frieren"
~~~

To try terminal playback:

~~~sh
./target/release/watch --inline "Frieren"
~~~

Inline playback requires MPV built with a compatible terminal video output driver and a terminal emulator that supports that output protocol. This is experimental on Android/Termux. Do not assume that `--inline` will automatically switch to an external player if it fails; automatic fallback is still pending.

## 6. Optional Android storage access

Only grant Termux shared-storage access if you need to access files in shared storage:

~~~sh
termux-setup-storage
~~~

Accept Android's permission prompt. This creates Termux storage links under `~/storage`. Granting storage access is not required merely to clone and build the project in Termux's private home directory.

## 7. Troubleshooting

### `am` or activity launch fails

Check that the player app is installed and updated. The current implementation invokes Android's `am` command and uses a fixed activity name for MPV/VLC. A generic Android app chooser is not implemented yet.

### The player opens but video does not start

Test the same URL in the player app if possible. The source may require a referer or may supply separate audio/subtitle streams. Those features are not guaranteed to transfer through Android's activity integration.

### Inline playback fails

Use ordinary external playback (without `--inline`). Terminal graphics support depends on the Termux terminal and the MPV build. Automatic fallback is not implemented in the current playback flow.

### Build fails

Retry with the current Termux packages:

~~~sh
pkg update && pkg upgrade
pkg install rust git clang
cargo build --release
~~~

If it still fails, include the Android version, device CPU architecture, Termux source/version, Rust version (`rustc --version`), the exact command, and the complete compiler error.

## 8. Current gaps

The following work remains before Android support should be considered complete:

1. Integrate playback-target selection into the actual CLI playback flow.
2. Implement and test fallback from inline playback to external playback when inline setup or process launch fails.
3. Add an implicit Android `VIEW` intent fallback instead of requiring a fixed MPV/VLC activity.
4. Make Android configuration, referer, audio, and subtitle handling reliable without hard-coded shared-storage paths.
5. Test search and playback on a physical Termux device across MPV and VLC.
6. Build the separate native Android app after extracting reusable application services from the CLI.
