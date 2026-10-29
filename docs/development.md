# Development guide

This guide describes the Rust implementation and Android stabilization workflow. `main` is the stable installation target; `android-support` is the Android stabilization branch. Watch is a fork of [pystardust/ani-cli](https://github.com/pystardust/ani-cli); see the README for project credit.

## Build and test

Install Rust 1.85 or newer, then run:

~~~sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
~~~

The installed executable is `watch`. Run it from source with `cargo run --release --bin watch -- "Frieren"`. To inspect command-line behavior without searching the provider, use `cargo run --bin watch -- --help` or `cargo run --bin watch -- --version`. On Linux, `watch` may conflict with the system process-monitoring utility; make sure Cargo’s binary directory is earlier in `PATH`.

## Source layout

| Path | Responsibility |
| --- | --- |
| `src/cli/` | Argument parsing, interactive menus, and selection. |
| `src/application.rs` | Source selection, episode-range rules, and cross-cutting use-case policies. |\n| `src/application_services.rs` | Reusable search, episode, stream-resolution, and history use cases intended for multiple frontends. |
| `src/domain/` | Anime, episode, stream, subtitle, and history types. |
| `src/providers/` | Parallel searches across English and Spanish sites, title/language grouping, episode extraction, stream resolvers, and AnimeSchedule requests. |
| `src/media/` | HLS parsing, playback arguments, terminal video backend selection, downloads, and external media-tool integration. |
| `src/infrastructure/` | Portable paths, configuration, and watch-history persistence. |
| `src/main.rs` | CLI orchestration and end-to-end application flow; migration to the shared service API is in progress. |
| `tests/unit/` | Unit-test bodies, mirrored by source module. Source files include them with `#[path]` so tests retain access to private implementation details. |

## Provider changes

Provider structure is external and can change without notice. Before changing parsing behavior:

1. Preserve a small response fixture for the case being handled.
2. Add or update a parser test that uses the fixture without network access.
3. Keep HTTP request/status/error handling separate from pure HTML or JSON parsing where practical.
4. Make malformed responses and verification pages produce actionable errors rather than silently looking like an empty search result.

Do not make the test suite depend on the live provider. Keep provider changes narrowly scoped and document behavior that varies by platform.

## Media and platform changes

Playback and downloading delegate work to external programs. Test command-line argument construction without launching the program, including URLs, referers, separate audio, subtitles, and custom flags. Keep test bodies in `tests/unit/`; do not put `#[test]` functions in `src/`. Guard platform-specific behavior explicitly and document runtime prerequisites in the tutorial and Android guide.

## Pull request checklist

See [contributing.md](contributing.md) for quality gates and [pull_request_template.md](pull_request_template.md) for the review checklist.
