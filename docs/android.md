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

If the package is not published yet or Cargo cannot find this project’s version, install it directly from the repository’s `main` branch:

```sh
cargo install --git https://github.com/gabrielfranicevich/watch --branch main
```

For future updates, run:

```sh
watch -U
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

`watch` creates or updates the file when needed. You do not need to enter anime URLs manually.

To make MPV-Android load it:

1. Open MPV-Android.
2. Go to **Settings → Advanced → Edit mpv.conf** (the wording may vary by version).
3. Add this line, keeping any other existing options:

   ```text
   include=/storage/emulated/0/mpv/mpv.config.mp4
   ```

4. Save the changes and restart MPV-Android if needed.

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
watch -U
```

Then continue using Watch as usual:

```sh
watch anime-title
```

Keep the `include` line in MPV-Android’s `mpv.conf`. If you reinstall the player, reset its configuration, or clear its data, make sure the line is still there.
