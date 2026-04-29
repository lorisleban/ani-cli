# Contributing

## Ground rules

- Keep the Rust app as the source of truth for behavior and docs.
- Run formatting and lint checks before opening a PR.
- Update `README.md` or `hacking.md` when the user-facing flow or architecture changes.
- Avoid adding new dependencies unless they clearly simplify the code or unlock something we cannot do cleanly otherwise.

## Before you open a PR

Run:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

If you changed runtime behavior, also smoke-test the TUI manually:

- search for a show
- open an episode list
- launch playback in at least one player
- verify history still updates

## Code areas

- `src/main.rs`: input handling and screen orchestration
- `src/app.rs`: app state transitions
- `src/api.rs`: scraping and provider parsing
- `src/player.rs`: external player launch behavior
- `src/db.rs`: history persistence
- `src/ui/`: presentation layer only

Try to keep responsibilities separated that way.

## Issue reports

Good bug reports usually include:

- OS and terminal emulator
- player used (`mpv`, `IINA`, `VLC`)
- the anime and episode involved
- whether the problem happens in sub, dub, or both
- logs or `/tmp/ani-cli-*` artifacts from `ANI_CLI_DEBUG_API=1` when the issue is provider/API-related

## Feature requests

This fork is centered on the Rust TUI. Requests are most helpful when they describe how the feature should fit the current interactive flow, not just parity with the original shell script.
