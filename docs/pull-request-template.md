# Pull request checklist

## Change summary

- What behavior changes?
- Which issue or failure case does this address?

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all-targets`
- [ ] Added regression coverage for the bug or feature.
- [ ] Tests use local fixtures instead of the live provider.
- [ ] Updated the README/tutorial/migration notes when behavior changes.
- [ ] Checked platform-specific player/downloader arguments where relevant.

## User-visible checks (when relevant)

- [ ] Search and episode selection.
- [ ] Playback and subtitle/audio forwarding.
- [ ] Downloading and output path.
- [ ] Watch history and continuation.
- [ ] `--help`, man page, and CLI option documentation.
