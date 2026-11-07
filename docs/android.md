# Install and Configure Watch on Android

Use these steps to install `watch` on Android with Termux, configure MPV-Android, and play anime streams with the required options.

## 1. Install the apps

Install these two apps:

- **Termux:** the terminal where you install and run `watch`. The F-Droid version is recommended: [Termux on F-Droid](https://f-droid.org/packages/com.termux/).
- **MPV-Android:** the video player, available from [MPV-Android on GitHub](https://github.com/mpv-android/mpv-android/releases).

Open Termux and update its packages:

```sh
pkg update && pkg upgrade
```

Install the dependencies:

```sh
pkg install rust git
```

Confirm Cargo is available:

```sh
cargo --version
```

## 2. Install Watch

Install the executable from crates.io with Cargo:

```sh
cargo install watch
```

For the Android stabilization build, install directly from the `android-support` branch. On Termux, use this command to avoid Cargo's `Text file busy (os error 26)` temporary-build failure by using a directory under `$HOME` and a single build job:

```sh
mkdir -p "$HOME/.tmp" && TMPDIR="$HOME/.tmp" CARGO_BUILD_JOBS=1 cargo install --git https://github.com/gabrielfranicevich/watch --branch develop --locked --force
```

For future updates, run:

```sh
watch -U develop
```

Confirm the command is available:

```sh
watch --help
```

## 3. Configure MPV-Android

MPV-Android needs to load the configuration file that `watch` generates for each playback. The file contains stream options such as `Referer` and external subtitles.

Create the shared directory from Termux:

```sh
mkdir -p /storage/emulated/0/mpv
```

`watch` updates per-playback options in this file:

```text
/storage/emulated/0/mpv/mpv.config.mp4
```

`watch` creates or updates this file on each playback, including clearing stale stream-specific options when the next stream does not need them. It also adds the following include to `/storage/emulated/0/mpv/mpv.conf` if it is missing, while preserving other existing settings:

```text
include=/storage/emulated/0/mpv/mpv.config.mp4
```

If playback opens but immediately stops on a stream that needs a `Referer`, check that MPV-Android can read shared storage and that the include is present in its active configuration. The exact configuration UI varies by MPV-Android version. Watch now stops before launching MPV-Android when it cannot prepare the shared per-stream configuration, rather than silently launching without required headers or tracks.

## 4. Grant the required permissions

Check MPV-Android’s permissions under **Android Settings → Apps → MPV-Android → Permissions**. Grant the media-file permissions that are available.

Permission names and options vary by Android and player version. Some devices do not show an option called “Allow access to manage all files.”

If Termux cannot create or update files in the shared directory, run `termux-setup-storage`, grant Android’s permission, and try again.

## 5. Test playback

Search for an anime from Termux:

```sh
watch beerus
```

Select a result and episode. If Android shows an app chooser, select MPV.

To test MPV and `termux-open` independently, try this public demo stream:

```sh
termux-open --chooser \
  --content-type 'video/*' \
  'https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8'
```

If MPV plays the video, opening media with `termux-open` works. Anime stream availability also depends on the provider.

## 6. Update Watch

You do not need to repeat the installation. Run:

```sh
watch -U develop
```

Then continue using Watch as usual:

```sh
watch anime-title
```

Keep the `include` line in MPV-Android’s `mpv.conf`. `watch` will add it when it can access shared storage, but reinstalling or resetting MPV-Android may reset the player's active configuration.

`--inline` and its alias `--here` are not supported in Termux: detecting MPV output drivers does not prove that the terminal can render their graphics protocols. Omit these options to use Android external playback. Inline playback remains a desktop feature and depends on the terminal and MPV build.


## Native Android application

The Kotlin/Jetpack Compose application and Rust JNI bridge are documented in the [native Android guide](native-android.md). The app is a separate frontend; the Termux CLI workflow below remains supported.

## 7. Compatibility and known limitations

The Android CI job cross-compiles the Rust CLI for `aarch64-linux-android`; it does not run MPV-Android, validate Android intents on a device, or prove that a provider's stream can be played. There is not yet a maintained device/version compatibility matrix. Record the Android version, device model, Termux version, MPV-Android release, player choice, provider, and stream type when reporting a playback result.

The currently supported handoff paths are:

- **MPV-Android:** Watch opens the stream through `termux-open` using the `video/*` MIME type. Referer, external audio, and external subtitle options are written to `/storage/emulated/0/mpv/mpv.config.mp4`, which is included by the shared `mpv.conf`.
- **VLC Android:** Watch uses the Android VIEW intent and can pass an external subtitle through `subtitles_location`. The public intent does not expose a documented separate-audio extra, so selected external audio cannot be applied.
- **Runtime track switching:** not supported by either Android player integration. Select available tracks before launching playback.

Before reporting a failure, test a public HLS stream with the standalone `termux-open` command above, then test a provider stream. If only provider streams fail, compare their referer requirement and media URL type. If the player opens but does not play, verify the shared-storage permission for Termux, the active MPV-Android configuration path, the include line, and the generated per-stream options. Do not assume that a successful ARM64 CI build proves physical-device playback.

Physical-device acceptance remains necessary for MPV-Android and VLC, including HLS and MP4 streams, referer-dependent streams, external audio and subtitles, repeated playback with different options, history, and supported Android versions.
