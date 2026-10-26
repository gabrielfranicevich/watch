# Contribution Guidelines

## Pull Requests

- Run `cargo fmt --all -- --check`.
- Run `cargo check --all-targets`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo test --all-targets`.
- Keep changes small, testable, and compatible with the documented CLI behavior.
- Update the README or migration notes when user-visible behavior changes.
- Avoid new dependencies unless necessary.
- If fixing an issue, link the issue or describe the failure case in a regression test.

### Coding Tips

- Prefer focused modules with explicit responsibilities over adding more orchestration to `main.rs`.
- Keep domain logic independent of HTTP, terminal I/O and external processes.
- Handle platform-specific process arguments explicitly and test argument construction without launching a player.
- Use local fixtures for network-dependent behavior so tests do not depend on the live provider.

### AI Policy

- AI is not allowed to write code comments
- Do not AI generate PR descriptions, it's insulting.
  Write one human sentence, or even leave it blank instead.
- Low effort PRs will be closed, especially ones violating the PR description and code comment rule.
- Add the AI model as a coauthor
- Using LLMs as a better search engine is okay
- Using LLMs to remember syntax and idioms is okay
- Using LLMs to refactor into posix compliance is okay
- Opening fully AI generated PRs is not okay
- A carefully steered and reviewed agentic PR does not constitute fully AI generated, see low effort.
- Be cautious that LLMs tend to be overly verbose, while the ani-cli codebase prefers brevity

### Email

If you don't have a GitHub account, or you prefer to privately contribute, there is an alternative.
Send email patches or pull requests to `port19@port19.xyz`.
For privacy, mind committer name and email and tell me if there is anything I should keep in mind.

Read [the docs](https://git-scm.com/book/en/v2/Distributed-Git-Contributing-to-a-Project) for git request-pull, git format-patch, [git send-email](https://git-send-email.io/) and [git diff](https://stackoverflow.com/questions/4610744/can-i-get-a-patch-compatible-output-from-git-diff).

The guidelines that apply to pull requests also apply to email patches.

## Issues

- Use the issue templates
- When requesting a feature, check it hasn't been [rejected](https://github.com/pystardust/ani-cli/issues/523) previously
- Provide screenshot if applicable

## How else can I help?

- Join the [discord](https://discord.gg/aqu7GpqVmR)
- Take part in troubleshooting and testing
- Star the repo
- Follow the maintainers
