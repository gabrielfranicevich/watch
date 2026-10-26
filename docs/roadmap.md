# Rust migration roadmap

## Goals

- Replace the shell implementation with a maintainable Rust CLI without silently dropping existing behavior.
- Keep changes small, testable, and buildable at every step.
- Preserve the existing shell script as a behavioral reference until Rust covers the required workflows.
- Keep provider-specific scraping separate from the domain, CLI, history, and media integrations.

## Repository baseline

The default branch remains the upstream-compatible shell implementation. Rust work is isolated on `develop`. Existing branches were reviewed before creating it:

- `curl-impersonate-windows`: one Windows-specific curl-impersonate change.
- `docs/readme-update`: README troubleshooting/dependency documentation.
- `fix-download-args`: downloader argument forwarding and quoting fixes.
- `next`: backup provider work and related documentation/workflow changes.
- `port19x-patch-1`: MacPorts documentation.
- `master`: current baseline.

These branches are not merged automatically; their changes are independent of the Rust migration and should be evaluated separately before release.

## Quality gates

Run locally before each commit:

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Rustfmt uses its standard four-space indentation; it does not support a stable two-space indentation setting. Keep Rust source formatted by rustfmt instead of introducing a competing formatter.

## Migration sequence

1. Establish Rust CI and a minimal Cargo project.
2. Add domain models and a thin CLI.
3. Add configuration and portable path handling.
4. Add regression fixtures for legacy history, then implement safe history migration.
5. Implement interactive selection.
6. Isolate HTTP and provider scraping; test parsers using local fixtures.
7. Add stream/subtitle resolution.
8. Add process abstraction, player adapters, and downloader support.
9. Add platform checks, harden external input handling, and document failure modes.
10. Update packaging and installation only after the Rust executable covers required workflows.
11. Remove the shell implementation only after feature parity and clean-checkout CI are verified.

## Architecture boundaries

- `cli/`: arguments, interactive prompts, output and exit codes.
- `application/`: use-case orchestration.
- `domain/`: domain types and rules; no HTTP, terminal or process dependencies.
- `providers/`: provider-specific requests and parsing.
- `media/`: playback, subtitles and downloads.
- `infrastructure/`: HTTP client, filesystem, configuration, process execution and platform detection.

Avoid empty placeholder modules and speculative abstractions. Introduce modules when there is real behavior to put in them.

## Known audit items to address

- Legacy history migration uses `IFS="\\t"`, which does not represent a literal tab in POSIX shell. Add a regression fixture before changing or replacing this behavior.
- The history update path uses a shared `.new` temporary file and needs concurrency/interruption-safe handling.
- The self-updater downloads executable code without an independent signature check. Reassess the update model before replacing it; do not claim transport TLS alone authenticates the artifact.
- HTML/JSON/HLS parsing is brittle when coupled to provider markup. Add fixture-based tests before refactoring.
- CI currently checks shell style and repository conventions but does not provide functional regression coverage for the full user workflow.

## Completion criteria

- The four Cargo quality gates pass on a clean checkout.
- Required CLI options, search, episode listing, streams, playback, downloads, subtitles and history have explicit acceptance tests or documented platform limitations.
- Legacy history is preserved or migrated with a tested procedure.
- Missing external tools and network/provider failures produce actionable errors.
- Supported platforms are documented and checked.
- Installation and packaging point to the Rust binary.
- The shell implementation is removed only after the above criteria are met.
