# Rust project roadmap

## Current status

The active development branch is `develop`; `main` remains the stable installation target. Android ARM64 CLI compatibility remains part of the CI matrix. This repository is a fork of [pystardust/ani-cli](https://github.com/pystardust/ani-cli), whose original shell project remains available upstream.

The Rust CI matrix checks Linux, macOS, and Windows with formatting, compilation, Clippy, tests, and CLI help/version smoke tests. It also cross-checks the Android ARM64 target. Provider regression tests use local fixtures rather than relying on the live service. These checks validate repository code, not the continued availability of external websites or media players.

## Android support status

- [x] Add a pure playback-target policy with unit tests.
- [x] Add MPV-Android playback integration and generate shared MPV configuration for per-stream options.
- [x] Add VLC Android playback selection.
- [x] Define a platform-independent automatic/explicit playback policy with regression tests.
- [x] Add an ARM64 Android cross-compilation check to CI.
- [ ] Fix and validate MPV-Android external playback in Termux, including URI, referer, audio, and subtitle handoff; validate playback, subtitles, and history across Android versions and physical devices.
- [ ] Define and test the Android behavior of `--here`, which currently works only on desktop platforms.
- [x] Introduce a reusable library-level application service API for search, episode retrieval, stream resolution, and history, with contract tests for input validation and history updates.
- [x] Route CLI search, episode retrieval, and stream resolution through the shared application services; history persistence retains the CLI-compatible path.
- [x] Add a JNI bridge and validate locked Android-target dependency resolution.
- [x] Build a Kotlin/Jetpack Compose app shell that calls the shared Rust services.
- [x] Add native Android playback with stream headers, subtitles, and separate audio where supported.
- [x] Add Android app and bridge CI with emulator instrumentation coverage.
- [ ] Complete native playback acceptance on physical Android devices; emulator instrumentation does not validate live provider streams or physical-device media compatibility.

## Completed development milestones

- [x] Replace unconditional `cargo install --force` updates with a persistent Git checkout and Cargo target cache; skip a revision only after its installation succeeds.
- [x] Stage updater installations separately, replace the installed executable only after Cargo succeeds, and retain the previous executable if replacement fails.
- [x] Remove interrupted updater staging directories after 24 hours while leaving recent or active staging directories untouched.
- [x] Preserve Cargo package-tracking metadata across staged installs and roll back the executable if metadata synchronization fails.
- [x] Cover updater clone, fetch, invalid-checkout recovery, incremental target reuse, staged-install listing, and Windows executable-lock behavior with isolated integration tests.
- [x] Represent audio and subtitle languages independently, preserve unknown values, and group catalog results by the pair.
- [x] Detect default audio/subtitle language tags from HLS master manifests and explicit HiAnime/HTML subtitle metadata; print the resolved pair before playback.
- [x] Expose declared HLS audio and subtitle renditions, including alternatives, through `ResolvedMedia` for shared frontends.
- [x] Associate renditions with the selected HLS video variant and exclude unrelated audio/subtitle groups.
- [x] Offer pre-playback selection for alternate URL-backed HLS audio and subtitle tracks, retaining the provider default as the first option.
- [x] Pass selected external track URLs to MPV-compatible players and VLC desktop.
- [x] Track language evidence provenance for provider defaults, search metadata, HLS manifests, embed metadata, and HTML subtitle tracks; retain audio and subtitle provenance independently during media resolution.
- [x] Add catalog integrations for Gnula, PelisPedia, PelisPlus, HDToday, and Pelis7 with fixture-based parser tests and source-specific limitations.
- [x] Add independent `--audio` and `--subtitles` catalog filters, including tests for combined matching and unknown metadata.
- [x] Document language filtering and derive default HLS audio/subtitle metadata through the shared rendition parser.
- [x] Add deterministic end-to-end coverage for catalog grouping, language variants, and source filters.
- [x] Cover player startup, subtitle HTTP, and history persistence failure paths.
- [x] Build release archives for Linux x86_64/ARM64, macOS Intel/Apple Silicon, and Windows x86_64 through GitHub Actions.
- [x] Add Homebrew, Debian, and RPM package definitions and document release channels.

## Remaining work

- Add runtime track switching after playback starts and extend language metadata extraction to non-HLS manifests. Current CLI track selection occurs before the player launches and only includes URL-backed renditions declared by HLS playlists.
- Verify Pelis7's live routes and add a site-specific episode parser only after its current page structure is observable.
- Extend failure-path coverage to additional downloader fallback and native Android device workflows without depending on live providers.
- Publish the first tagged release from `main`, then validate the Homebrew formula and Linux package recipes against that release.
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
