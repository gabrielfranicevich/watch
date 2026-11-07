# Release channels and supported platforms

## Release channels

| Channel | Source | Distribution |
| --- | --- | --- |
| Stable | Version tags on `main`, named `vMAJOR.MINOR.PATCH` | GitHub Releases with platform archives, Android APK, and SHA-256 checksums |
| Development | Pushes to `develop` | Rolling GitHub pre-release (`develop`) plus Actions artifacts for 14 days |
| Pull requests | Pull requests targeting `main` or `develop` | GitHub Actions artifacts for 14 days |

The release workflow builds the desktop targets and native Android application on pushes to `main` and `develop`, pull requests targeting either branch, tag pushes, and manual dispatches. The resulting archives and APK are available from the corresponding workflow run for 14 days. For pushes to `develop`, a rolling GitHub pre-release named **Watch Development** is updated after the Rust and Markdown workflows and all release build jobs succeed. For a `vMAJOR.MINOR.PATCH` tag, the workflow publishes a stable GitHub Release after those same checks pass, together with `SHA256SUMS`.

The release workflow waits for the Rust quality and Markdown workflows for the exact same commit SHA. If either workflow fails, is cancelled, or does not start within the timeout, publication is skipped. The release is also gated on every platform build and the Android APK build succeeding. A stable release tag must match the Cargo package version and should be created from `main`. Pull requests and manual dispatches build and store CI artifacts but do not publish a release.

GitHub Packages is not used for these downloads: its registries require supported package formats such as npm, Maven, NuGet, RubyGems, or Docker images. The CLI archives and APK are ordinary binary release assets, so GitHub Actions artifacts and GitHub Releases are the appropriate distribution channels.

## Release binaries

| Operating system | Architecture | Target | Archive |
| --- | --- | --- | --- |
| Linux (glibc 2.39+) | x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux (glibc 2.39+) | ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS | Intel | `x86_64-apple-darwin` | `.tar.gz` |
| macOS | Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` |
| Android | ARM64 and x86_64 | Rust JNI bridge + Kotlin application | Universal debug `.apk` |

Each desktop archive contains the `watch` executable (or `watch.exe`), the man page, the README, and the license. The Android APK contains native Rust/JNI libraries for ARM64 physical devices and x86_64 emulators. It targets Android API 26 or newer.

Desktop release archives are built on Ubuntu 24.04, macOS, and Windows. The Linux binaries require glibc 2.39 or newer; they are not musl-static builds. Windows ARM64, 32-bit targets, and other Unix platforms are not currently release targets.

The Android workflow currently publishes a debug-signed APK for CI and manual testing. Each GitHub-hosted runner generates its own debug signing key, so a later APK from another workflow run may not install as an in-place update over an earlier build. Uninstall the previous debug build before installing a replacement. A stable Android distribution channel requires a dedicated release keystore configured as repository Actions secrets; that signing material is intentionally not committed to the repository.

The release contains `SHA256SUMS`; verify a downloaded file before using it:

~~~sh
sha256sum -c SHA256SUMS
~~~

## Installation

### Linux and Windows archives

Download the archive matching the operating system and architecture from the [GitHub Releases page](https://github.com/gabrielfranicevich/watch/releases). Development builds are listed under the **Watch Development** pre-release. Extract it and place the executable in a directory on `PATH`. On Linux, the executable is named `watch`; the system process-monitoring utility may use the same name, so ensure the intended binary appears first on `PATH`. On Windows, add the directory containing `watch.exe` to `PATH`.

### Android APK

Download `watch-<version>-android-universal-debug.apk` from the [GitHub Releases page](https://github.com/gabrielfranicevich/watch/releases) or the `watch-android` artifact from the workflow run. Install it with Android's package installer or:

~~~sh
adb install watch-<version>-android-universal-debug.apk
~~~

The debug APK is intended for development and testing. See the [native Android application guide](native-android.md) for architecture, prerequisites, and physical-device acceptance coverage.

### Homebrew

Install the formula from this repository's tap:

~~~sh
brew install gabrielfranicevich/watch/watch
~~~

The formula builds from a pinned upstream Git revision and requires Rust at build time. Update the pinned revision and formula version together for each stable release. MPV is recommended; FFmpeg is required for downloads and separate audio/video muxing. Prebuilt release archives include `SHA256SUMS` for checksum verification.

### Debian packages

The Debian source package definition is in `debian/`. Build a binary package from a source checkout with Debian packaging tools installed:

~~~sh
dpkg-buildpackage -us -uc -b
~~~

The package installs the executable as `watch-anime`, rather than `watch`, to avoid replacing the `watch` command supplied by procps. It installs the corresponding `watch-anime(1)` manual page. MPV or VLC and FFmpeg are recommended runtime tools; yt-dlp and Syncplay are optional.

### RPM packages

The RPM spec is `packaging/rpm/watch-anime.spec`. Build it with `rpmbuild` and the Rust toolchain installed:

~~~sh
rpmbuild -ba packaging/rpm/watch-anime.spec
~~~

Like the Debian package, the RPM installs the executable as `watch-anime` to avoid a file conflict with procps.

## Android

Android is supported separately from the desktop release matrix:

- The Rust CLI is cross-checked for `aarch64-linux-android` in CI, but a standalone Android CLI release archive is not currently published.
- The native Android application is built for ARM64 physical devices and x86_64 emulators. CI validates the build; the separate Rust workflow runs emulator instrumentation tests.
- The debug APK is a testing build and does not have a stable release signing key. It may need to be uninstalled before replacing it with an APK from another workflow run.
- Termux users should follow the [Android / Termux guide](android.md). Install the CLI from source using the supported Rust/Termux toolchain.

## Runtime requirements and limitations

Playback requires a supported external media player. FFmpeg is required for downloads and for muxing streams with separate audio; yt-dlp is an optional preferred downloader for supported single-stream sources. Optional selection tools and intro-skipping integrations are described in the README.

Provider websites, media hosts, player integrations, and stream formats are third-party dependencies. A successful release build does not guarantee that every provider or stream is available, nor does it validate every operating-system/player combination. Report reproducible compatibility issues with the OS version, architecture, Watch version, provider, player, and relevant redacted logs.
