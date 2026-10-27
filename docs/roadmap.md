# Rust project roadmap

## Current status

The active Rust implementation is developed on `develop`. The historical shell implementation remains on `main`; this document describes the Rust branch.

The Rust CI matrix checks Linux, macOS, and Windows with formatting, compilation, Clippy, tests, and CLI help/version smoke tests. The `android-support` branch adds an ARM64 Android cross-check and initial Termux playback fallback. Provider regression tests use local fixtures rather than relying on the live service. These checks validate repository code, not the continued availability of external websites or media players.

## Android support status

- [x] Add a pure playback-target policy with unit tests.
- [ ] Implement and test automatic fallback from inline playback to external playback on Android.
- [ ] Add a generic Android VIEW intent as a fallback when explicit MPV/VLC integration is unavailable.
- [x] Add an ARM64 Android cross-compilation check to CI.
- [x] Run the Android ARM64 cross-compilation job successfully.
- [ ] Validate playback, subtitles, referer-dependent streams, and history on a physical Termux device.
- [ ] Extract the Rust application workflow from `src/main.rs` before building a native Android UI.
- [ ] Build a separate Kotlin/Jetpack Compose app that reuses the Rust domain/provider layer.

## Remaining work

- Verify locked dependency resolution from a clean checkout with `cargo check --all-targets --locked` and `cargo test --all-targets --locked`. Regenerate `Cargo.lock` only if Cargo reports that it needs an update, and keep `--locked` out of updater/install commands until validation passes.
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
