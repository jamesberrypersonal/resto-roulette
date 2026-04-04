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

See `docs/initial-design-doc.md` for full detail.

- **`src/main.rs`** — CLI entry point via `clap` derive macros; orchestrates the pipeline with concurrent API fetching (`buffer_unordered(10)`).
- **`src/config.rs`** — Resolves configuration precedence: `--flag` > env var > `~/.resto-roulette/config.toml`. Note: `--home` short flag is `-H` (not `-h`, which clap reserves for `--help`).
- **`src/parse/`** — Three parser paths dispatched by file extension and CSV headers:
  - `geojson.rs` — Google Takeout GeoJSON (includes coordinates; coordinate order is `[lng, lat]` per GeoJSON spec — the parser swaps to lat/lng).
  - `csv.rs` — Auto-detects two formats: simple `name,address` CSV and Google Maps shared-list export (`Title,Note,URL,Tags,Comment`). The shared-list format uses the place name as the routing address.
- **`src/routing/`** — HTTP client (`client.rs`) for Google Routes API. Queries all four travel modes (walk/bike/transit/drive) per restaurant in parallel via `tokio::join!`. The `X-Goog-FieldMask: routes.duration` header is required to stay on the Basic SKU. Response durations are protobuf strings (`"720s"`) requiring custom deserialization.
- **`src/cache/sqlite.rs`** — SQLite cache at `~/.resto-roulette/cache.db`. Cache key is SHA-256(name + `\x00` + address) + SHA-256(home address) + mode. Default TTL: 1 week. Expired entries are evicted at startup.
- **`src/bucket.rs`** — Assigns each restaurant to exactly one bucket (the nearest it qualifies for). Mode eligibility per bucket: Near (walk/bike/transit), Mid (bike/transit only), Far (bike/transit/drive).
- **`src/picker.rs`** — Random selection from each bucket; takes generic `R: Rng` for deterministic tests.
- **`src/display.rs`** — `pretty` (colored terminal) and `json` output formatters.
- **`src/error.rs`** — Unified `AppError` enum via `thiserror`. `main.rs` wraps it in `anyhow::Result` for context-rich error messages.

## Key Design Decisions

- **Bucketing**: A restaurant lands in exactly one bucket — the nearest one it qualifies for. This prevents close-by spots from dominating every slot.
- **API key**: Read from `--api-key` flag > `GOOGLE_MAPS_API_KEY` env var > config file.
- **Home address**: Same precedence — `--home` flag > `RESTO_HOME` env var > config file.
- **`--dry-run`**: Uses stale cache without any API calls.
- **`rusqlite` with `bundled` feature**: Compiles SQLite from source for zero system dependencies.
- **Shared lists**: Google Takeout only exports lists you own. Shared lists from other users must be exported via the Google Maps shared-list CSV export.

## Release Notes

User-facing changes are tracked in `RELEASE_NOTES.md` at the repo root. When adding a feature or fixing a bug, add a bullet under the `## Unreleased` section (create it at the top if it doesn't exist). Keep entries concise and user-facing in tone.

## Testing Approach

- Unit tests live in each module under `#[cfg(test)]`.
- Integration tests in `tests/` cover parsing, bucketing, and picker logic using fixture files.
- Test fixtures (`tests/fixtures/`) include `sample.geojson`, `sample.csv`, `sample_maps_export.csv`, and `routes_response.json`.
- Seed the RNG when testing the picker for deterministic results (`StdRng::seed_from_u64`).
