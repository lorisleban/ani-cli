# Hacking on ani-cli

This fork is a Rust TUI application, not a shell-script menu wrapper anymore. If you want to modify scraping, playback, or navigation behavior, start from the Rust modules below.

## Architecture map

- `src/main.rs`: tiny binary entrypoint that calls the runtime
- `src/lib.rs`: crate module map
- `src/runtime/`: terminal setup/cleanup and the event loop
- `src/app.rs`: shared app state, history refresh, navigation stack, playback bookkeeping
- `src/domain/`: core types shared across app, UI, providers, persistence, and player code
- `src/api.rs`: compatibility facade that exposes the active anime catalog client
- `src/providers/allanime/`: allanime/allanime-day requests, decoding, provider parsing, quality selection, and curl fallback transport
- `src/persistence/`: SQLite watch-history storage
- `src/services/`: service traits for anime catalog, history, and playback boundaries
- `src/player/`: player detection and detached launch logic
- `src/ui/`: rendering for home, search, detail, history, help, and now-playing screens

## Running locally

```sh
cargo run --release
```

Useful while iterating:

```sh
cargo fmt
cargo clippy -- -D warnings
cargo build
```

## Scraping flow

The Rust app still follows the same broad content pipeline as the older project, but the implementation lives in `src/api.rs` now.

1. `search_anime(query)` fetches matching shows
2. `episodes_list(show_id)` fetches available episodes for the selected show and mode
3. `get_episode_url(show_id, episode, quality)` resolves provider URLs and picks a stream
4. `player::launch_player(...)` hands the final URL to the configured external player

## Debugging API issues

Turn on API debugging with:

```sh
ANI_CLI_DEBUG_API=1 cargo run --release
```

That writes intermediate payloads and responses to files in `/tmp`, including:

- `ani-cli-search-request.json`
- `ani-cli-search.json`
- `ani-cli-episode-request.json`
- `ani-cli-episode.json`
- provider fallback `curl` dumps

This is the fastest way to understand whether a breakage is happening at:

- the GraphQL request layer
- the provider URL decoding layer
- the per-provider media-link parsing layer

## Modifying providers

Most provider-related work happens in `src/providers/allanime/`.

Places worth starting with:

- `mod.rs`: top-level search, episode list, and stream resolution
- `config.rs`: AllAnime host/header/query constants
- `transport.rs`: captcha detection plus curl fallback requests
- `decode_tobeparsed(...)`: provider payload decoding
- `fetch_provider_links(...)`: provider-specific media-link extraction

A safe workflow is:

1. capture failing payloads with `ANI_CLI_DEBUG_API=1`
2. write the smallest parser fix possible
3. verify `best`, `worst`, and exact quality matching still behave sensibly
4. confirm playback still launches through at least one player

## UI work

TUI rendering lives in `src/ui/`. Keep business logic out of these files when possible.

- input/state transitions belong in `src/main.rs` or `src/app.rs`
- rendering and layout belong in `src/ui/`
- theming helpers belong in `src/theme.rs`

## Persistence

History is stored in SQLite via `src/db.rs`. If you change stored fields or table shape, make sure existing users are considered before shipping the change.

## Scope notes

This repo still contains traces of its shell-script origin, but the Rust binary is the supported app. When in doubt, prioritize the current TUI architecture over compatibility with every legacy script flag.
