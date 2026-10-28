# Android / Termux installation guide

This guide covers the **watch CLI** on Android through [Termux](https://termux.dev/). It does not cover a native Android application: the repository does not currently contain an APK or Kotlin/Jetpack Compose app.

## Status and limitations

- The Rust CLI can be built for Android ARM64; GitHub Actions currently checks the `aarch64-linux-android` target.
- Runtime playback has **not** been verified on a physical Android device.
- The current Android player integration launches the MPV Android activity by default. The `--vlc` option selects the VLC Android activity.
- The CLI does **not yet** implement generic Android media-handler fallback or reliable inline-to-external fallback. The playback policy module has unit tests, but the policy is not integrated into the current playback flow.
- MPV playback writes per-stream referer/audio/subtitle settings and adds an `include=` line to MPV-Android's shared-storage `mpv.conf`. Protected streams will fail if MPV-Android cannot read that file. Grant **Allow access to manage all files** to MPV-Android and shared-storage access to Termux. Physical-device playback still needs verification.

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

## 6. Storage access for MPV playback

Protected streams can require a `Referer` header. On Android, watch writes those per-stream options to `/storage/emulated/0/mpv/mpv.config.mp4` and ensures that `/storage/emulated/0/mpv/mpv.conf` includes that file. This is needed because launching MPV with an Android intent cannot pass arbitrary MPV command-line flags.

Grant Termux shared-storage access:

~~~sh
termux-setup-storage
~~~

Accept Android's permission prompt. Then open **Android Settings → Apps → MPV → Permissions** (the labels vary by Android version) and grant **Allow access to manage all files** / **All files access** to MPV-Android. A general media/photos permission may not be sufficient to read `/storage/emulated/0/mpv/mpv.conf`.

watch preserves existing settings in `mpv.conf` and avoids adding the same include line twice. The stream's `Referer` is written to `mpv.config.mp4`; if MPV cannot read the include file, protected streams can open the player and then stop without playback.

This permission is not required merely to clone and build watch. If watch reports that it could not configure MPV, check both apps' storage permissions before troubleshooting the stream itself.

## 7. Troubleshooting

### `am` or activity launch fails

Check that the player app is installed and updated. The current implementation invokes Android's `am` command. MPV is launched using its documented package-based `VIEW` intent; VLC still uses a fixed activity name. A generic Android app chooser is not implemented yet.

### MPV opens but video does not start

Check the warning printed by watch. For streams that require a referer:

1. In Termux, run `termux-setup-storage` and accept the prompt.
2. Open Android Settings → Apps → MPV → Permissions.
3. Grant **Allow access to manage all files** / **All files access** to MPV-Android (`is.xyz.mpv`). A basic photos/media permission may not be enough.
4. Rebuild or update watch to the latest `android-support` branch and retry.

watch creates `/storage/emulated/0/mpv/mpv.conf` and includes the per-stream config automatically. If playback still fails after granting full file access, copy the complete Termux output, including the `Resolving episode...` and `Stream resolution:` messages; the remaining failure may be the provider stream rather than the Android intent.

If playback still fails, test the stream in MPV-Android itself and capture the complete Termux output. Some providers reject requests independently of the referer, and separate audio/subtitle handling can still vary by stream.

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
4. Verify MPV referer/audio/subtitle configuration on a physical Termux device and replace the hard-coded shared-storage path if device testing exposes compatibility issues.
5. Test search and playback on a physical Termux device across MPV and VLC.
6. Build the separate native Android app after extracting reusable application services from the CLI.
