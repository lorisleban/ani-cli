# Legacy Shell Script

The repository still contains a top-level `ani-cli` POSIX shell script from the project lineage this fork came from.

For this fork:

- the maintained application is the Rust binary built from `src/`
- the terminal UI is the primary user experience
- documentation, CI, and contributor expectations should target the Rust app

The shell script is being kept only as:

- historical context
- a migration reference while Rust behavior continues to settle
- a source of scraping/playback ideas that may still be useful during rewrites

It should not be treated as the supported runtime entrypoint for this fork.

If we eventually remove or relocate it, that should happen in a deliberate follow-up so anyone still depending on it has a clear migration path.
