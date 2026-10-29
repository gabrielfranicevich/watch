# Rust project roadmap

## Current status

The stable installation target is `main`; Android stabilization is being developed on `android-support`. This repository is a fork of [pystardust/ani-cli](https://github.com/pystardust/ani-cli), whose original shell project remains available upstream.

The Rust CI matrix checks Linux, macOS, and Windows with formatting, compilation, Clippy, tests, and CLI help/version smoke tests. It also cross-checks the Android ARM64 target. Provider regression tests use local fixtures rather than relying on the live service. These checks validate repository code, not the continued availability of external websites or media players.

## Android support status

- [x] Add a pure playback-target policy with unit tests.
- [x] Add MPV-Android playback integration and generate shared MPV configuration for per-stream options.
- [x] Add VLC Android playback selection.
- [x] Define a platform-independent automatic/explicit playback policy with regression tests.
- [x] Add an ARM64 Android cross-compilation check to CI.
- [ ] Validate playback, subtitles, referer-dependent streams, and history across Android versions and physical Termux devices.
- [x] Introduce a reusable library-level application service API for search, episode retrieval, stream resolution, and history, with contract tests for input validation and history updates.
- [ ] Migrate the CLI orchestration in `src/main.rs` to call the shared services instead of duplicating their workflows.
- [ ] Add a JNI/UniFFI proof of concept and validate locked Android-target dependency resolution.
- [ ] Build a Kotlin/Jetpack Compose app shell that calls the shared Rust services.
- [ ] Add native Android playback with stream headers, subtitles, and separate audio where supported.
- [ ] Add Android app and bridge CI, then validate installation and playback on a physical device.

## Remaining work

- Verify locked dependency resolution from a clean checkout with `cargo check --locked --all-targets`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked --all-targets`. Regenerate `Cargo.lock` only if Cargo reports that it needs an update, and keep `--locked` out of updater/install commands until validation passes.
- Expand acceptance tests for error paths and user-visible workflows without depending on the live provider or launching external media tools.
- Publish prebuilt binaries and update third-party package definitions only when release automation and platform validation are ready.
- Keep the README, tutorial, man page, and CLI help synchronized as options change.

## Validation

Follow the build and test instructions in the [development guide](development.md#build-and-test) and the quality gates in the [contributing guide](contributing.md#before-opening-a-pull-request).

## Release criteria

- All four Cargo quality gates pass from a clean checkout.
- Important CLI options and workflows have regression coverage or a documented platform limitation.
- Legacy history remains readable and migration behavior is tested.
- Provider/network failures and missing external tools produce actionable errors.
- Installation and packaging instructions point to the Rust binary and accurately state which platforms/packages are supported.
- Live provider availability and compatibility with third-party media tools are described as external dependencies, not guaranteed behavior.
