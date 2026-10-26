# Contributing

The Rust CLI is developed on `develop`. Keep changes focused and preserve user-visible behavior documented in the README and tutorial.

## Before opening a pull request

Run all four quality gates:

~~~sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
~~~

GitHub Actions runs these checks, plus CLI help/version smoke tests, on Linux, macOS, and Windows.

## Code and tests

- Keep argument parsing, interactive menus, and selection in `src/cli/`.
- Put domain types and rules in `src/domain/`; avoid HTTP, process, or terminal dependencies there.
- Keep provider requests and response parsing in `src/providers/`.
- Put HLS parsing, playback, subtitles, and downloads in `src/media/`.
- Keep portable paths, configuration, and history persistence in `src/infrastructure/`.
- Put cross-cutting use-case rules in `src/application.rs`.
- Add regression coverage with bug fixes. Prefer local HTTP fixtures and static response samples; tests must not depend on the live provider.
- Test player/downloader argument construction without launching real external programs.
- Keep Markdown documentation under `docs/`; the root README is the only Markdown file outside that directory.

## Pull requests

Describe the problem and behavior change, link related issues, and include the commands used to test it. Use the checklist in [pull-request-template.md](pull-request-template.md) when preparing a pull request.

Do not claim compatibility for a platform or player unless it has been tested. Provider markup and third-party tools can change independently of this repository.
