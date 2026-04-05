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

- **`src/main.rs`** — CLI entry point via `clap` derive macros; orchestrates the pipeline: parse → enrich (lazy, Places API) → filter closed → fetch travel times (`buffer_unordered(10)`) → bucket → pick → display.
- **`src/config.rs`** — Resolves configuration precedence: `--flag` > env var > `~/.resto-roulette/config.toml`. Note: `--home` short flag is `-H` (not `-h`, which clap reserves for `--help`).
- **`src/parse/`** — Three parser paths dispatched by file extension and CSV headers:
  - `geojson.rs` — Google Takeout GeoJSON (includes coordinates; coordinate order is `[lng, lat]` per GeoJSON spec — the parser swaps to lat/lng).
  - `csv.rs` — Auto-detects two formats: simple `name,address` CSV and Google Maps shared-list export (`Title,Note,URL,Tags,Comment`). The shared-list format uses the place name as the routing address.
- **`src/places/`** — Google Places API (New) integration. Only active when `--open-now` is set (lazy enrichment).
  - `client.rs` — HTTP client for Text Search (first encounter, resolves name+address → place ID + hours) and Place Details (cheap refresh using cached place ID). `X-Goog-FieldMask` is set to Basic SKU fields only.
  - `models.rs` — API response types (deserialization) and domain structs (`PlaceDetails`, `WeeklyHours`, `HoursPeriod`, `DayTime`) with serde for cache serialization.
  - `hours.rs` — `is_open_at(hours, utc_offset_minutes, now_utc)` function. Uses the restaurant's own UTC offset (not system timezone) for correctness when travelling. Handles midnight rollover and the Saturday→Sunday week boundary.
- **`src/routing/`** — HTTP client (`client.rs`) for Google Routes API. Queries all four travel modes (walk/bike/transit/drive) per restaurant in parallel via `tokio::join!`. The `X-Goog-FieldMask: routes.duration` header is required to stay on the Basic SKU. Response durations are protobuf strings (`"720s"`) requiring custom deserialization.
- **`src/cache/sqlite.rs`** — SQLite cache at `~/.resto-roulette/cache.db`. Has two tables: `travel_times` (key: SHA-256(name+address) + SHA-256(home) + mode, TTL: 1 week) and `place_details` (key: SHA-256(name+address), TTL: 30 days). Both tables are evicted at startup. `Cache::open` takes separate TTL parameters for each table.
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
- **Lazy Places enrichment**: The Places API is only called when `--open-now` is active. Without it, the pipeline is identical to v1 — no extra API calls, no extra cost.
- **Fail-open on Places API**: If a restaurant's place details can't be fetched (API failure, no hours in response, dry-run with no cache), the restaurant is kept — never silently dropped due to an enrichment failure.
- **Places API same key**: `--open-now` uses the same `GOOGLE_MAPS_API_KEY` credential as Routes. The user must enable **Places API (New)** in their Google Cloud project. A 403 surfaces a clear error message pointing to this.

## Release Notes

User-facing changes are tracked in `RELEASE_NOTES.md` at the repo root. When adding a feature or fixing a bug, add a bullet under the `## Unreleased` section (create it at the top if it doesn't exist). Keep entries concise and user-facing in tone.

## Testing Approach

- Unit tests live in each module under `#[cfg(test)]`.
- Integration tests in `tests/` cover parsing, bucketing, and picker logic using fixture files.
- Test fixtures (`tests/fixtures/`) include `sample.geojson`, `sample.csv`, `sample_maps_export.csv`, `routes_response.json`, `places_text_search_response.json`, and `places_details_response.json`.
- Seed the RNG when testing the picker for deterministic results (`StdRng::seed_from_u64`).
