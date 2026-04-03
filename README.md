# resto-roulette

Random restaurant picker from your Google Maps saved list, bucketed by travel time from home.

You have a growing "want to try" restaurant list in Google Maps. When it's time to eat, decision fatigue kicks in. This tool picks three restaurants for you — one nearby, one mid-range, and one worth the trek.

```
$ resto-roulette --list "Want to Go.json"

🚶 Walk/Bike/Transit ≤15 min
   → Nouveau Palais (281 Rue Bernard O, Montréal, QC)   ~10 min

🚲 Bike/Transit 15–30 min
   → Hà (243 Rue De Bleury, Montréal, QC)   ~22 min

🚗 Bike/Transit/Car 30–60 min
   → Cabane à sucre Au Pied de Cochon (Saint-Benoît, QC)   ~47 min

Re-roll? [y/N]
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

The binary is at `target/release/resto-roulette`.

### 3. Get a Google Maps API key

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project and enable the **Routes API**
3. Go to **APIs & Services > Credentials** and create an API key
4. (Recommended) Restrict the key to only the Routes API

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
  -H, --home <HOME>          Home address (env: RESTO_HOME)
  -l, --list <LIST>          Path to restaurant list [default: saved_places.csv]
  -r, --reroll               Interactive re-roll mode
      --format <FORMAT>      Output format: pretty or json [default: pretty]
      --cache-ttl <HOURS>    Hours to cache travel times [default: 168]
      --dry-run              Use cached data only, no API calls
      --api-key <API_KEY>    Google Maps API key (env: GOOGLE_MAPS_API_KEY)
  -h, --help                 Print help
```

### Examples

```bash
# Basic usage
resto-roulette --list places.json

# JSON output (for piping to other tools)
resto-roulette --list places.json --format json

# Re-roll mode (press y to get new picks)
resto-roulette --list places.json --reroll

# Dry run (no API calls, uses whatever is cached)
resto-roulette --list places.json --dry-run

# Debug logging (see cache hits, API calls, etc.)
RUST_LOG=debug resto-roulette --list places.json
```

## How it works

1. **Parse** the input file (GeoJSON, simple CSV, or Google Maps export CSV)
2. **Check cache** — travel times are stored in `~/.resto-roulette/cache.db` (SQLite)
3. **Fetch** travel times from the Google Routes API for any cache misses (4 modes per restaurant, up to 10 restaurants concurrently)
4. **Bucket** each restaurant into the nearest tier it qualifies for:
   - **Near** (≤15 min): walking, biking, or transit
   - **Mid** (15–30 min): biking or transit
   - **Far** (30–60 min): biking, transit, or driving
5. **Pick** one random restaurant from each bucket
6. **Display** the results

Restaurants too far away (>60 min by any mode) are silently excluded. Empty buckets show a friendly message.

## Cost

The Google Routes API has a free tier of 10,000 requests per month. A typical run with 50 restaurants makes ~200 API calls, and results are cached for 1 week by default. You're unlikely to exceed the free tier with normal personal use. See the [design doc](docs/initial-design-doc.md#14-api-cost-analysis) for detailed cost analysis.

## Development

```bash
cargo check                      # fast type-check
cargo test                       # run all tests
cargo clippy -- -D warnings      # lint
cargo fmt                        # format
```

## License

MIT
