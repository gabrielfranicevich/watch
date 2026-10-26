# Rust project roadmap

## Current status

The active Rust implementation is developed on `develop`. The historical shell implementation remains on `main`; this documentation describes the Rust branch.

The Rust CI matrix currently checks Linux, macOS, and Windows with formatting, compilation, Clippy, tests, and CLI help/version smoke tests. Provider regression tests use local fixtures rather than relying on the live service. These checks validate the repository code, not the continued availability of external websites or media players.

## Architecture

- `src/cli/`: argument parsing, interactive menus, and selection.
- `src/application.rs`: use-case policies and episode-range rules.
- `src/domain/`: anime, episode, stream, subtitle, and history types.
- `src/providers/`: provider requests, response parsing, stream resolution, and AnimeSchedule integration.
- `src/media/`: HLS parsing, playback arguments, subtitles, and downloads.
- `src/infrastructure/`: configuration, portable paths, and history persistence.
- `src/main.rs`: CLI orchestration.

Keep provider-specific parsing separate from the domain and prefer small, testable changes. Avoid introducing abstractions without a concrete use case.

## Remaining work

- Regenerate and commit a complete `Cargo.lock` for reproducible locked builds before restoring `--locked` to updater/install instructions.
- Expand acceptance tests for error paths and user-visible workflows without depending on the live provider or launching external media tools.
- Publish prebuilt binaries and update third-party package definitions only when release automation and platform validation are ready.
- Keep the README, tutorial, man page, and CLI help synchronized as options change.

## Quality gates

Run locally before submitting changes:

~~~sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
~~~

GitHub Actions runs these gates and CLI help/version smoke tests on Linux, macOS, and Windows.

## Release criteria

- All four Cargo quality gates pass from a clean checkout.
- Important CLI options and workflows have regression coverage or a documented platform limitation.
- Legacy history remains readable and migration behavior is tested.
- Provider/network failures and missing external tools produce actionable errors.
- Installation and packaging instructions point to the Rust binary and accurately state which platforms/packages are supported.
- Live provider availability and compatibility with third-party media tools are described as external dependencies, not guaranteed behavior.
