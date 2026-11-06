# Development guide

This guide describes the Rust implementation and Android stabilization workflow. `develop` is the active development branch; `main` is the stable installation target. Android-specific CI continues to validate the ARM64 CLI target. Watch is a fork of [pystardust/ani-cli](https://github.com/pystardust/ani-cli); see the README for project credit.

## Build and test

Install Rust 1.85 or newer, then run:

~~~sh
cargo fmt --all -- --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
~~~

The installed executable is `watch`. Run it from source with `cargo run --release --bin watch -- "Frieren"`. To inspect command-line behavior without searching the provider, use `cargo run --bin watch -- --help` or `cargo run --bin watch -- --version`. On Linux, `watch` may conflict with the system process-monitoring utility; make sure Cargo’s binary directory is earlier in `PATH`.

## Source layout

| Path | Responsibility |
| --- | --- |
| `src/cli/` | Argument parsing, interactive menus, and selection. |
| `src/application.rs` | Source selection, episode-range rules, and cross-cutting use-case policies. |
| `src/application_services.rs` | Reusable search, episode, stream-resolution, and history use cases intended for multiple frontends. |
| `src/domain/` | Anime, episode, stream, subtitle, history, normalized audio/subtitle language pairs, and language-evidence source types. |
| `src/providers/` | Parallel searches across anime and movie/TV sites, title/audio-subtitle grouping, provider-specific episode extraction, stream resolvers, language-evidence provenance, and AnimeSchedule requests. |
| `src/media/` | HLS parsing, playback arguments, terminal video backend selection, downloads, and external media-tool integration. |
| `src/infrastructure/` | Portable paths, configuration, and watch-history persistence. |
| `src/main.rs` | CLI orchestration and end-to-end application flow; search, episode retrieval, and stream resolution use the shared service API. |
| `tests/unit/` | Unit-test bodies, mirrored by source module. Source files include them with `#[path]` so tests retain access to private implementation details. |

## Provider changes

Provider structure is external and can change without notice. Consult [provider support](providers.md) for the current enabled source list, the meaning of language-pair values, and known limitations. Before changing parsing behavior:

1. Preserve a small response fixture for the case being handled.
2. Add or update a parser test that uses the fixture without network access.
3. Keep HTTP request/status/error handling separate from pure HTML or JSON parsing where practical.
4. Make malformed responses and verification pages produce actionable errors rather than silently looking like an empty search result.

Do not make the test suite depend on the live provider. Keep provider changes narrowly scoped and document behavior that varies by platform.

## Media and platform changes

Playback and downloading delegate work to external programs. Test command-line argument construction without launching the program, including URLs, referers, separate audio, subtitles, and custom flags. Keep test bodies in `tests/unit/`; do not put `#[test]` functions in `src/`. Guard platform-specific behavior explicitly and document runtime prerequisites in the tutorial and Android guide.


## HLS track selection

When a resolved HLS master playlist declares multiple URL-backed audio or subtitle renditions for the selected video variant, the CLI offers a separate selection menu for each available track type before playback. The provider default is kept as the first choice. Manifests without alternate renditions continue directly to playback. Renditions from groups not referenced by the chosen video variant are excluded.

`--audio` and `--subtitles` remain catalog source filters; they do not pick a track. Track selection is a playback-time pre-launch choice. The selected URLs and language metadata are passed to MPV-compatible players and VLC desktop. Desktop MPV-compatible players with MPV IPC support can also add or select external tracks during playback. Android MPV-Android receives stream options through its shared configuration, while Android VLC accepts external subtitles but has no documented separate-audio intent extra; neither Android integration currently supports runtime track switching. The external player remains responsible for loading rendition URLs; this does not guarantee support from every player build or streaming host. HLS subtitle playlists are not downloaded as standalone VTT files.

## Updater behavior

The `--update` path reuses a persistent Git checkout and Cargo target directory. An invalid or interrupted checkout is discarded and cloned again; a valid checkout fetches the requested branch. The target directory is reused with incremental compilation enabled. Before `cargo install --locked`, the updater seeds a unique staging Cargo root with the active root's `.crates.toml` and `.crates2.json` tracking metadata. After a successful staged build, Watch replaces the executable and synchronizes the updated tracking files back to the active Cargo root, preserving registrations for other installed packages. If metadata synchronization fails after the executable is replaced, the previous executable is restored and the update fails without advancing the installed-revision marker. The marker is written only after both operations succeed, so a failed build or replacement is retried on the next update. Staging directories left behind by an interrupted update are removed on a later run when they are more than 24 hours old; newer staging directories are left untouched. Unix replacement uses a same-filesystem rename. On Windows, the previous executable is moved to a temporary backup and restored if the staged executable cannot be moved into place; if the running executable cannot be renamed, the update fails without removing it. The updater does not delete the previous executable before a successful staged install.

## Pull request checklist

See [contributing.md](contributing.md) for quality gates and [pull_request_template.md](pull_request_template.md) for the review checklist.
