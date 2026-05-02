# Release Notes

## Unreleased

- Internal: workspace refactor — pipeline extracted into `resto-roulette-core`. CLI behavior unchanged.
- New `resto-roulette-server` binary: serves daily restaurant picks as a [TRMNL](https://usetrmnl.com) e-ink plugin. Exposes `GET /healthz` and `GET /trmnl` (token-authenticated) returning JSON with one pick per bucket. Configure via `~/.resto-roulette/server.toml`.

## v1.1.0

- Add per-bucket candidate browsing: press `Tab`/`l`/`→` in the TUI to scroll through all restaurants in the selected bucket and pick one manually
- Add full-screen explorer mode (`--explore` flag, or press `e` in the TUI): browse all buckets with search (press `/`) and sort cycling (press `s`: Name → Time → Cuisine), with a detail panel for the highlighted restaurant
- Dependency updates

## v1.0.0

- v1 release
- Upgraded dependencies

## v0.3.0

- Replace text re-roll prompt with an interactive TUI: navigate buckets with `↑↓`/`jk`, re-roll a single slot with `r`, re-roll all with `R`, accept with `Enter`, quit with `q`. Automatically falls back to single-pick output when using `--one-shot`, `--format json`, or piping.
- Add `--open-now` flag to filter out currently-closed restaurants (requires Places API (New) enabled on your Google Cloud key)
- Add `--cuisine` flag to filter recommendations by cuisine type (e.g. `--cuisine japanese` or `--cuisine "japanese,korean"`)
- Add `exclude_cuisines` config option to permanently exclude cuisine types (e.g. `exclude_cuisines = ["fast food"]`)
- Cuisine type is now shown inline in pretty output when available (e.g. `→ Hà (Vietnamese · 243 Rue De Bleury)`)

## v0.2.0

- Pretty output now shows the transport mode used for each time estimate (e.g. `~8 min by walking`)
- Re-roll is now the default; use `--one-shot` / `-o` to pick once and exit
- Fixed bug where `--cache-ttl` CLI flag was incorrectly overridden by the config file value
- Removed unused code (dead geocoding method, unused error variant)

## v0.1.1

- Upgraded dependencies
- Updated config file to optionally include list path
- Fixed bug where drive-only restaurants under 30 min were incorrectly placed in the Far (30–60 min) bucket

## v0.1.0

- Initial release
