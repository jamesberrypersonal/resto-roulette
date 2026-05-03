# resto-roulette

Random restaurant picker from your Google Maps saved list, bucketed by travel time from home.

You have a growing "want to try" restaurant list in Google Maps. When it's time to eat, decision fatigue kicks in. This tool picks three restaurants for you — one nearby, one mid-range, and one worth the trek.

```
$ resto-roulette --list "Want to Go.json"

┌─ resto-roulette ─────────────────────────────────────────────┐
│                                                              │
│  🚶 Walk/Bike/Transit ≤15 min                               │
│  ► Nouveau Palais (Diner · 281 Rue Bernard O)                │
│    ~10 min by walking                                        │
│                                                              │
│  🚲 Bike/Transit 15–30 min                                  │
│  ► Hà (Vietnamese · 243 Rue De Bleury)                       │
│    ~22 min by transit                                        │
│                                                              │
│  🚗 Bike/Transit/Car 30–60 min                              │
│  ► Cabane à sucre Au Pied de Cochon (~47 min by car)         │
│                                                              │
│  ↑↓/jk Navigate  r Re-roll slot  R Re-roll all              │
│  Tab Browse  e Explore  Enter Accept  q Quit                 │
└──────────────────────────────────────────────────────────────┘
```

## Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Build

```bash
git clone https://github.com/your-user/resto-roulette.git
cd resto-roulette
cargo build --release
```

The binary is at `target/release/resto-roulette`. You can also build just the CLI crate:

```bash
cargo build --release -p resto-roulette-cli
```

### 3. Get a Google Maps API key

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project and enable the **Routes API**
3. If you want to use `--open-now` or `--cuisine`, also enable the **Places API (New)**
4. Go to **APIs & Services > Credentials** and create an API key

### 4. Configure

Set your API key and home address. Pick any of these methods (highest priority wins):

**Environment variables** (recommended):
```bash
export GOOGLE_MAPS_API_KEY="AIza..."
export RESTO_HOME="123 Rue Saint-Denis, Montréal, QC"
```

**Config file** (`~/.resto-roulette/config.toml`):
```toml
api_key = "AIza..."
home = "123 Rue Saint-Denis, Montréal, QC"
exclude_cuisines = ["fast food", "pizza"]  # optional: always exclude these
```

**CLI flags** (highest priority, overrides the above):
```bash
resto-roulette --api-key "AIza..." --home "123 Rue Saint-Denis, Montréal, QC" --list places.json
```

## Getting your restaurant list

### Google Takeout (best for lists you own)

1. Go to [takeout.google.com](https://takeout.google.com)
2. Deselect all, then check only **Maps (your places)**
3. Download and unzip — each saved list is a separate `.json` file inside `Maps/`

```bash
resto-roulette --list "Maps/Want to Go.json"
```

### Shared-list CSV export (for lists shared by others)

Google Takeout only includes lists you own. For shared lists, export from Google Maps as a CSV. The tool auto-detects the `Title,Note,URL,Tags,Comment` format:

```bash
resto-roulette --list shared_list.csv
```

### Manual CSV

Create a simple two-column CSV:

```csv
name,address
Hà,243 Rue De Bleury Montréal QC
Nouveau Palais,281 Rue Bernard O Montréal QC
```

## Usage

```
resto-roulette [OPTIONS]

Options:
  -H, --home <HOME>                    Home address or lat,lng (env: RESTO_HOME)
  -l, --list <LIST>                    Path to exported list file (CSV or GeoJSON)
  -o, --one-shot                       Pick once and exit without prompting to re-roll
      --format <FORMAT>                Output format: pretty or json
      --cache-ttl <CACHE_TTL>          Hours to cache travel times (default: 168)
      --dry-run                        Show buckets without API calls (uses cache only)
      --api-key <API_KEY>              Google Maps API key (env: GOOGLE_MAPS_API_KEY)
      --open-now                       Only recommend restaurants that are currently open
      --places-cache-ttl <HOURS>       Hours to cache place details (default: 720)
      --cuisine <CUISINE>              Filter to specific cuisines, comma-separated (e.g. "japanese,korean")
      --explore                        Launch directly into the full-screen restaurant explorer
  -h, --help                           Print help
```

### Examples

```bash
# Basic usage — launches interactive TUI
# Navigate: j/k or ↑↓   Re-roll slot: r   Re-roll all: R   Accept: Enter   Quit: q
# Browse all candidates for a bucket: Tab (or l/→)
# Full-screen explorer with search and sort: e (or --explore flag)
resto-roulette --list places.json

# Launch directly into the restaurant explorer (browse, search, sort all candidates)
resto-roulette --list places.json --explore

# Only show restaurants that are currently open
resto-roulette --list places.json --open-now

# Filter to a specific cuisine
resto-roulette --list places.json --cuisine japanese

# Filter to multiple cuisines
resto-roulette --list places.json --cuisine "japanese,korean"

# Combine: open now + specific cuisine
resto-roulette --list places.json --open-now --cuisine vietnamese

# Pick once and exit (no re-roll prompt)
resto-roulette --list places.json --one-shot

# JSON output (for piping to other tools)
resto-roulette --list places.json --format json

# Dry run (no API calls, uses whatever is cached)
resto-roulette --list places.json --dry-run

# Debug logging (see cache hits, API calls, etc.)
RUST_LOG=debug resto-roulette --list places.json
```

## How it works

1. **Parse** the input file (GeoJSON, simple CSV, or Google Maps export CSV)
2. **Enrich** (only with `--open-now` or `--cuisine`) — fetches opening hours and cuisine types from the Google Places API and caches them for 30 days
3. **Filter** — closed restaurants (with `--open-now`) and/or cuisine mismatches (with `--cuisine` or `exclude_cuisines`)
4. **Check cache** — travel times are stored in `~/.resto-roulette/cache.db` (SQLite)
5. **Fetch** travel times from the Google Routes API for any cache misses (4 modes per restaurant, up to 10 restaurants concurrently)
6. **Bucket** each restaurant into the nearest tier it qualifies for:
   - **Near** (≤15 min): walking, biking, or transit
   - **Mid** (15–30 min): biking or transit
   - **Far** (30–60 min): biking, transit, or driving
7. **Pick** one random restaurant from each bucket
8. **Display** the results

Restaurants too far away (>60 min by any mode) are silently excluded. Empty buckets show a friendly message.

## Cost

The Google Routes API has a free tier of 10,000 requests per month. A typical run with 50 restaurants makes ~200 API calls, and results are cached for 1 week by default. You're unlikely to exceed the free tier with normal personal use.

`--open-now` and `--cuisine` both use the Google Places API to enrich restaurant data, adding a one-time cost of ~$0.03/restaurant (Google Places Text Search) on first use. Results are cached for 30 days, so subsequent runs are free. See the [design doc](docs/phase-2-design-doc-places-tui.md#7-api-cost-analysis) for detailed cost analysis.

## Development

```bash
cargo check --workspace          # fast type-check
cargo test --workspace           # run all tests
cargo clippy --workspace -- -D warnings  # lint
cargo fmt --all                  # format
```

## Server (TRMNL plugin)

`resto-roulette-server` is a sibling binary that serves the picker as a [TRMNL](https://usetrmnl.com) e-ink plugin. On each request it runs the full pipeline and returns one pick per bucket as JSON, which TRMNL renders via a Liquid template on the device. See [`docs/phase-3-design-doc-server-workspace.md`](docs/phase-3-design-doc-server-workspace.md) for the full design.

### Configure

Create `~/.resto-roulette/server.toml`:

```toml
home       = "123 Rue Saint-Denis, Montréal, QC"
list_path  = "/var/lib/resto-roulette/list.geojson"
api_key    = "AIza..."           # overridden by GOOGLE_MAPS_API_KEY env var
auth_token = "long-random-hex"   # overridden by RESTO_AUTH_TOKEN env var
bind_addr  = "127.0.0.1:8080"
```

`api_key` and `auth_token` can be kept out of the file and supplied via environment variables instead.

### Run locally

```bash
cargo run -p resto-roulette-server
curl -sf http://localhost:8080/healthz
curl -sf 'http://localhost:8080/trmnl?token=<RESTO_AUTH_TOKEN>' | jq .
```

`/healthz` requires no auth. `/trmnl` accepts the token via query param (`?token=`) or `X-Auth-Token` header.

### JSON response

```json
{
  "generated_at": "2026-05-02T08:00:00Z",
  "near": { "name": "Hà", "address": "243 Rue De Bleury", "duration_minutes": 12, "mode": "walk", "cuisine": "vietnamese" },
  "mid":  { "name": "Schwartz's", "address": "3895 Saint-Laurent Blvd", "duration_minutes": 22, "mode": "bike", "cuisine": null },
  "far":  { "name": "Joe Beef", "address": "2491 Notre-Dame St W", "duration_minutes": 38, "mode": "drive", "cuisine": null }
}
```

Empty buckets render as `null`. `cuisine` is `null` when not cached from a previous CLI run with `--cuisine` or `--open-now`.

### Cross-compile for Raspberry Pi

```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-musl -p resto-roulette-server
scp target/aarch64-unknown-linux-musl/release/resto-roulette-server pi@homepi:/usr/local/bin/
```

Reference deployment files are in `deploy/`:

- [`deploy/systemd/resto-roulette-server.service`](deploy/systemd/resto-roulette-server.service) — systemd unit (uses `EnvironmentFile=/etc/resto-roulette/secrets.env` for secrets)
- [`deploy/cloudflared.config.yml`](deploy/cloudflared.config.yml) — Cloudflare Tunnel ingress template

The TRMNL Private Plugin URL becomes `https://resto.<your-domain>/trmnl?token=<RESTO_AUTH_TOKEN>`. See [`docs/trmnl-liquid-template.md`](docs/trmnl-liquid-template.md) for the Liquid template to paste into the plugin UI.

## License

MIT
