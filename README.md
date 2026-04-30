# ani-cli

A Rust rewrite of `ani-cli` with a full terminal UI built on `ratatui` and `crossterm`.

Maintained in this fork by [@lorisleban](https://github.com/lorisleban). The Rust TUI rewrite in this repository is your project surface here, not the original upstream shell script maintainers'.

This fork is no longer the original shell-script-first project. The supported entrypoint is the Rust binary in `src/`, and the app is designed around an interactive TUI for searching shows, picking episodes, tracking watch history, and handing playback off to an external player.

## What it does

- Search anime from inside the terminal UI
- Browse episode lists in a grid or long-form list
- Toggle between sub and dub mode
- Resume from local watch history
- Launch playback in `mpv`, `IINA`, or `VLC`
- Persist history in a local SQLite database

## Current scope

This rewrite is intentionally narrower than the upstream shell script.

- The Rust app is TUI-first. It does not currently expose the old flag-driven workflow from the shell script.
- Features like downloader flows, syncplay, self-update flags, and legacy menu integrations are not wired into the Rust app yet.
- The legacy `ani-cli` shell script may still exist in the repository as migration/reference material, but it is not the primary app.

## Platform status

Right now this codebase is still Unix-first, but Windows-native builds are now part of the supported path.

- `macOS`: supported, with automatic `IINA` detection when available
- `Linux`: supported
- `WSL`: possible, but not a polished target yet
- `Windows native`: supported for the Rust TUI build, with native `mpv` / `VLC` launch handling

## Requirements

Build-time:

- Rust toolchain (`rustup`, `cargo`)

Runtime:

- One media player:
  - `mpv`
  - `IINA` on macOS
  - `VLC`
- `curl`
  - used as a fallback for some API/provider requests

## Install

### Download a release binary

The easiest path for most users is a GitHub release asset.

Current planned release targets:

- `ani-cli-linux-x86_64.tar.gz`
- `ani-cli-macos-x86_64.tar.gz`
- `ani-cli-macos-aarch64.tar.gz`
- `ani-cli-windows-x86_64.zip`

After downloading:

```sh
tar -xzf ani-cli-*.tar.gz
chmod +x ani-cli
./ani-cli
```

If you want it on your `PATH`:

```sh
mkdir -p ~/.local/bin
mv ani-cli ~/.local/bin/
```

### Run from source

```sh
cargo run --release
```

### Build a release binary

```sh
cargo build --release
./target/release/ani-cli
```

### Install locally

```sh
make install-local
ani-cli
```

### Install globally

```sh
make install
ani-cli
```

### Install with Cargo

If you already use Rust tooling, you can install directly from the repo:

```sh
cargo install --git https://github.com/lorisleban/ani-cli
```

## Usage

Launch the app with:

```sh
ani-cli
```

Stable CLI entrypoints:

```sh
ani-cli --help
ani-cli --version
ani-cli doctor
```

The main playback workflow is still fully interactive and TUI-first.

## TUI controls

Global:

- `/`: open search from anywhere
- `g h`: go home
- `g s`: go search
- `g w`: go history
- `g p`: go now playing
- `?`: open help
- `Q` or `Ctrl-C`: quit
- `G`: jump to bottom
- `g g`: jump to top

Home:

- `j` / `k`: move through continue-watching items
- `Enter` or `r`: resume selected show
- `s`: open search
- `w`: open history
- `d`: toggle sub/dub mode

Search:

- Type to search
- `Enter`: open selected title
- `Backspace`: edit query
- `Esc`: go back

Episode detail:

- `h` / `j` / `k` / `l`: move through episodes
- `Enter` or `p`: play selected episode
- `d`: reload episode list in sub/dub mode
- `Esc`: go back

Now playing:

- `n` or `l`: play next episode
- `p` or `h`: play previous episode
- `r`: replay current episode
- `s`: jump back to episode picker
- `Esc`: return to detail view

History:

- `j` / `k`: move
- `x`: delete selected history item
- `X`: clear history
- `Esc`: go back

## Player behavior

The app auto-detects a player in this order:

1. `IINA` on macOS
2. `mpv`
3. `VLC`

Playback is launched as a detached process so the TUI can keep running.

## Data and debugging

History is stored in a SQLite database under your platform data directory in an `ani-cli/history.db` folder.

If you need API debugging output, run the app with:

```sh
ANI_CLI_DEBUG_API=1 cargo run --release
```

That writes request/response snapshots to `/tmp/ani-cli-*`.

To inspect local setup and player detection:

```sh
ani-cli doctor
```

## Development

Useful commands:

```sh
cargo fmt
cargo clippy --all-targets
cargo build --release
```

Or through the included `Makefile`:

```sh
make fmt
make check
make build
```

## Releases

Pushing a tag like `v0.1.0` triggers a GitHub Actions workflow that builds and uploads release archives for:

- Linux `x86_64`
- macOS `x86_64`
- macOS `aarch64` (Apple Silicon)
- Windows `x86_64`

## Source layout

- `src/main.rs`: tiny binary entrypoint
- `src/lib.rs`: crate module map
- `src/runtime/`: terminal lifecycle and the TUI event loop
- `src/app.rs`: app state, navigation, and playback bookkeeping
- `src/domain/`: shared domain types for anime, history, and playback
- `src/api.rs`: compatibility facade for the active anime provider
- `src/providers/allanime/`: AllAnime API integration, provider decoding, and fallback transport
- `src/persistence/`: SQLite-backed watch history implementation
- `src/services/`: service traits around catalog, history, and playback boundaries
- `src/player/`: external player detection and launch
- `src/ui/`: all TUI screens and chrome

## Notes for upstream users

If you came here expecting the original shell script behavior, read this fork as a separate app direction rather than a drop-in replacement for every upstream feature.

Upstream deserves credit for the original `ani-cli` concept and shell implementation. This repository specifically represents the Rust rewrite and TUI direction maintained in this fork by [@lorisleban](https://github.com/lorisleban).

The old top-level `ani-cli` shell script is kept only as historical/reference material for now. It is not the maintained app surface for this fork.

## Contributing

- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [hacking.md](./hacking.md)
- [LEGACY_SHELL_SCRIPT.md](./LEGACY_SHELL_SCRIPT.md)
- [disclaimer.md](./disclaimer.md)
