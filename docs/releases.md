# Release channels and supported platforms

## Release channels

| Channel | Source | Distribution |
| --- | --- | --- |
| Stable | Version tags on `main`, named `vMAJOR.MINOR.PATCH` | GitHub Releases with platform archives and SHA-256 checksums |
| Development | `develop` | Source installation only; commits are not stable release artifacts |

The release workflow validates that the tag version matches the Cargo package version. Pushes to `main` and `develop`, pull requests targeting either branch, and tag pushes build all supported release targets and store the archives as CI artifacts. A tag push additionally publishes the resulting archives to a GitHub Release. Manually dispatching the workflow builds and stores CI artifacts but does not publish a release.

No tagged binary release is published by the current development branch. The first stable release must be tagged from `main`.

## Release binaries

| Operating system | Architecture | Rust target | Archive |
| --- | --- | --- | --- |
| Linux (glibc 2.39+) | x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux (glibc 2.39+) | ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS | Intel | `x86_64-apple-darwin` | `.tar.gz` |
| macOS | Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

Each archive contains the `watch` executable (or `watch.exe`), the man page where applicable, the README, and the license. The release also contains `SHA256SUMS`; verify an archive before extracting it:

~~~sh
sha256sum -c SHA256SUMS
~~~

The Linux release binaries are built on Ubuntu 24.04 and require glibc 2.39 or newer; they are not musl-static builds. Windows ARM64, 32-bit targets, and other Unix platforms are not currently release targets.

## Installation

### Linux and Windows archives

Download the archive matching the operating system and architecture from the GitHub Releases page. Extract it and place the executable in a directory on `PATH`. On Linux, the executable is named `watch`; the system process-monitoring utility may use the same name, so ensure the intended binary appears first on `PATH`. On Windows, add the directory containing `watch.exe` to `PATH`.

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

- The Rust CLI is cross-checked for `aarch64-linux-android` in CI, but an Android CLI release archive is not currently published.
- The native Android application is built and instrumented in CI. Emulator tests validate app startup and the packaged JNI/Rust bridge; they do not establish physical-device playback compatibility.
- Termux users should follow the [Android / Termux guide](android.md). Install from source using the supported Rust/Termux toolchain.

## Runtime requirements and limitations

Playback requires a supported external media player. FFmpeg is required for downloads and for muxing streams with separate audio; yt-dlp is an optional preferred downloader for supported single-stream sources. Optional selection tools and intro-skipping integrations are described in the README.

Provider websites, media hosts, player integrations, and stream formats are third-party dependencies. A successful release build does not guarantee that every provider or stream is available, nor does it validate every operating-system/player combination. Report reproducible compatibility issues with the OS version, architecture, Watch version, provider, player, and relevant redacted logs.
