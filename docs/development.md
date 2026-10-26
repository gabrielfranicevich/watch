# Development guide

This guide describes the Rust implementation on `develop`. The historical shell version remains on `main`; old shell-specific reverse-engineering notes do not describe the active source tree.

## Build and test

Install Rust 1.85 or newer, then run:

~~~sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
~~~

Run the CLI from source with `cargo run --release -- "Frieren"`. To inspect command-line behavior without searching the provider, use `cargo run -- --help` or `cargo run -- --version`.

## Source layout

| Path | Responsibility |
| --- | --- |
| `src/cli/` | Argument parsing, interactive menus, and selection. |
| `src/application.rs` | Source selection, episode-range rules, and cross-cutting use-case policies. |
| `src/domain/` | Anime, episode, stream, subtitle, and history types. |
| `src/providers/` | HiAnime and AnimeSchedule requests, stream resolution, and response parsing. |
| `src/media/` | HLS parsing, playback arguments, downloads, and external media-tool integration. |
| `src/infrastructure/` | Portable paths, configuration, and watch-history persistence. |
| `src/main.rs` | CLI orchestration and end-to-end application flow. |

## Provider changes

Provider structure is external and can change without notice. Before changing parsing behavior:

1. Preserve a small response fixture for the case being handled.
2. Add or update a parser test that uses the fixture without network access.
3. Keep HTTP request/status/error handling separate from pure HTML or JSON parsing where practical.
4. Make malformed responses and verification pages produce actionable errors rather than silently looking like an empty search result.

Do not make the test suite depend on the live provider. Keep provider changes narrowly scoped and document behavior that varies by platform.

## Media and platform changes

Playback and downloading delegate work to external programs. Test command-line argument construction without launching the program, including URLs, referers, separate audio, subtitles, and custom flags. Guard platform-specific behavior explicitly and document runtime prerequisites in the tutorial.

## Pull request checklist

See [contributing.md](contributing.md) for quality gates and [pull_request_template.md](pull_request_template.md) for the review checklist.
