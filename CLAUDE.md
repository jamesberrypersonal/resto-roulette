# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`resto-roulette` is a Rust CLI tool that randomly recommends restaurants from your Google Maps saved list, bucketed into three travel-time tiers (≤15 min, 15–30 min, 30–60 min) from a home address. It calls the Google Routes API for travel times and caches results in a local SQLite database.

## Commands

```bash
cargo build            # debug build
cargo build --release  # release build (single static binary)
cargo check            # fast type-check without compiling (preferred during dev)
cargo test             # all unit + integration tests
cargo test <name>      # run a single test by name substring
cargo fmt              # format code
cargo fmt --check      # check formatting (used in CI)
cargo clippy -- -D warnings  # lint (all warnings are errors in CI)
cargo doc --open       # browse generated docs
```

## Architecture

The planned module layout (see `docs/initial-design-doc.md` for full detail):

- **`src/main.rs`** — CLI entry point via `clap` derive macros; reads flags/env vars and calls into `lib.rs`.
- **`src/config.rs`** — Resolves configuration precedence: `--flag` > env var > `~/.resto-roulette/config.toml`.
- **`src/parse/`** — Two parsers (`geojson.rs`, `csv.rs`) that both produce `Vec<Restaurant>`. GeoJSON input includes coordinates (no geocoding needed); CSV triggers geocoding via Google Geocoding API.
- **`src/routing/`** — HTTP client (`client.rs`) for Google Routes API (`routes.googleapis.com`). Queries all four travel modes (walk/bike/transit/drive) per restaurant. Hand-written request/response structs (no official Rust SDK).
- **`src/cache/sqlite.rs`** — SQLite cache at `~/.resto-roulette/cache.db`. Cache key is SHA-256(name + address) + SHA-256(home address) + mode. Default TTL: 1 week.
- **`src/bucket.rs`** — Assigns each restaurant to exactly one bucket (the nearest it qualifies for). Bucket boundaries are at 15 min and 30 min.
- **`src/picker.rs`** — Random selection from each bucket; seed the RNG for deterministic tests.
- **`src/display.rs`** — `pretty` (colored terminal) and `json` output formatters.
- **`src/error.rs`** — Unified `AppError` enum via `thiserror`. Empty-bucket errors are non-fatal (print a friendly message, continue).

## Key Design Decisions

- **Bucketing**: A restaurant lands in exactly one bucket — the nearest one it qualifies for. This prevents close-by spots from dominating every slot.
- **API key**: Read from `--api-key` flag > `GOOGLE_MAPS_API_KEY` env var > config file.
- **Home address**: Same precedence — `--home` flag > `RESTO_HOME` env var > config file.
- **`--dry-run`**: Uses stale cache without any API calls.
- **`rusqlite` with `bundled` feature**: Compiles SQLite from source for zero system dependencies.

## Testing Approach

- Unit tests live in each module under `#[cfg(test)]`.
- Integration tests in `tests/` use `wiremock` to mock the Google Routes API.
- Property-based tests via `proptest` cover bucketing invariants and picker deduplication.
- Test fixtures (`tests/fixtures/`) include `sample.geojson`, `sample.csv`, and a recorded `routes_response.json` for snapshot testing.
- Seed the RNG when testing the picker for deterministic results.
