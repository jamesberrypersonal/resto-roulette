# Design Exploration: Automated List Ingestion

> **Note:** This is an exploratory document examining potential approaches to automating restaurant list ingestion. It is not a committed roadmap item — implementation timeline and scope are undecided.

## Problem

resto-roulette requires a local file (`--list path/to/file.csv`) containing the restaurant list. For shared Google Maps lists — lists created by other users — the current workflow is:

1. Open Google Maps in a browser
2. Navigate to the shared list
3. Export as CSV (Title, Note, URL, Tags, Comment)
4. Save the file locally
5. Pass it to the CLI via `--list`

This process must be repeated every time someone adds or removes a restaurant from the shared list. It's the highest-friction part of using the tool, and the only step that can't be automated with the current architecture.

### Why this is hard

Google Maps Platform has **no API for accessing saved or shared lists**. The Places API supports search and detail lookups, but not list enumeration. Google Takeout exports only lists the user owns — not shared lists. There is no programmatic Takeout API for triggering exports on demand. This rules out the most obvious approaches and shapes the alternatives below.

---

## Approaches Analyzed

### 1. Scrape the shared list URL

**Concept:** The user provides a shared list URL (e.g. `https://www.google.com/maps/placelists/list/...`). The app fetches the page and extracts restaurant data from embedded JS blobs or HTML.

**Assessment:** Google Maps is a JavaScript-rendered SPA. The list data is not present in the initial HTML — it's loaded dynamically via XHR after JavaScript execution. The initial page does contain some data blobs in `<script>` tags (serialized protobuf wrapped in JS callbacks like `AF_initDataCallback`), but these are undocumented, unversioned, and change without notice.

**Verdict: Rejected.**
- Fragile — Google changes frontend data structures regularly; breakage is a matter of "when," not "if"
- Violates Google Maps ToS Section 3.2.3 (scraping prohibition)
- High implementation cost for low long-term reliability

### 2. Headless browser automation

**Concept:** Spawn a headless Chromium instance (via `headless_chrome` or `fantoccini` crate), navigate to the shared list URL, wait for JS rendering, and scrape the DOM.

**Verdict: Rejected.**
- Requires Chrome/Chromium installed on the user's system
- Adds enormous dependency weight, breaking the single-binary design goal
- Slow (browser startup) and flaky
- Still subject to the same ToS concerns as approach 1

### 3. Google Sheets as an intermediary

**Concept:** The restaurant list is maintained in a Google Sheet. The app reads it via the Google Sheets API (using the existing API key for public sheets).

**Assessment:** Technically straightforward — the Sheets API is stable, well-documented, and the existing API key works for publicly-viewable sheets. No new dependencies needed.

**Verdict: Rejected.**
- Does not solve the core problem — it shifts the manual work upstream. Someone must now manually update a spreadsheet whenever the Google Maps list changes, which is more burdensome than the current CSV export.
- The list owner may not want to maintain a parallel data source.

### 4. Reverse-engineer the Google Maps internal API

**Concept:** When you view a shared list in Google Maps, the browser makes XHR requests to Google's backend (e.g. `https://www.google.com/maps/rpc/...`) with protobuf-encoded payloads. We could reverse-engineer these endpoints and call them directly from the CLI.

**Assessment:** This is more stable than HTML scraping because these are actual API endpoints that the Maps frontend depends on — Google can't change them as freely as they change HTML structure. Some open-source projects have partially reverse-engineered these endpoints.

**Verdict: Risky, not recommended as a primary approach.**
- Undocumented — could break at any time, though likely less frequently than HTML structure changes
- Still violates ToS (accessing the service in an unauthorized manner)
- Protobuf payloads are complex to parse and construct without the `.proto` definitions
- Could be considered as a fallback if better options prove infeasible, but carries ongoing maintenance risk

### 5. Google Takeout (owned lists only)

**Concept:** Google Takeout exports owned lists as GeoJSON (the richest format, with coordinates). The app already parses this format perfectly.

**Assessment:** Takeout only includes lists the user **owns** — shared lists from other users are not included. The user would need to duplicate the shared list to make it their own, losing automatic sync when the original owner makes changes.

Additionally, Takeout has no programmatic API:
- **Manual export:** User visits takeout.google.com, selects Maps data, triggers download. Must be repeated manually.
- **Scheduled export:** Takeout supports periodic exports delivered to Google Drive, but the minimum frequency is **every 2 months** — far too infrequent for a list that changes regularly.

**Verdict: Not viable for the shared-list use case.**
- Only works for owned lists (shared lists excluded)
- No API for on-demand export
- 2-month minimum schedule is too coarse
- Duplicating a shared list breaks sync with the original

### 6. Google Drive API + Takeout pipeline

**Concept:** Schedule Takeout exports to Google Drive, then use the Drive API (with OAuth) to find and download the latest export automatically.

**Assessment:** This chains two mechanisms — Takeout for the export, Drive API for the download. The Drive API part is straightforward (well-documented, stable, OAuth-based).

**Verdict: Not viable — inherits all of Takeout's limitations.**
- Still bound by the 2-month schedule
- Still excludes shared lists
- The Drive API solves the download step but can't fix the upstream constraints

### 7. Chrome extension (Recommended)

**Concept:** A Chrome extension that runs on Google Maps pages extracts restaurant data from the rendered DOM when the user views a shared list, then sends it to the CLI tool.

**Assessment:** This sidesteps the "no API for lists" problem entirely. The browser has already done the JavaScript rendering and authentication. The extension simply reads what's already visible on the page — the same data the user sees. This is fundamentally different from scraping: the extension operates within the browser's normal execution context, not by making unauthorized requests to Google's servers.

**Verdict: Recommended — the most practical path to automation.** Detailed analysis follows.

---

## Chrome Extension: Detailed Analysis

### How it works

1. The user installs the extension (one-time setup)
2. The user opens the shared list in Google Maps (something they likely already do to browse restaurants)
3. The extension detects it's on a list page and extracts restaurant data from the rendered DOM — names, addresses, and potentially place IDs from data attributes or internal page structures
4. The extracted data is sent to the CLI tool via one of the transfer mechanisms below

### Data extraction

When a Google Maps shared list is fully rendered, the DOM contains structured elements for each place in the list, including:
- **Place name** (visible text)
- **Address / location description** (visible text)
- **Place URL** (link href, which contains an encoded place ID — the same `!1s...` format already seen in the CSV export URL column)

The extension would query these DOM elements using standard selectors. While DOM structure can change with Google Maps UI updates, the extension can be updated independently of the CLI tool, and DOM changes are typically visible and easy to debug (unlike server-side protobuf changes).

### Data transfer mechanisms

The extension needs to get extracted data from the browser to the CLI. Three viable mechanisms, in order of seamlessness:

#### Option A: Native Messaging (most seamless)

Chrome's [Native Messaging API](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) allows extensions to communicate with locally-installed programs via stdin/stdout.

- The CLI registers as a native messaging host (a one-time setup step that writes a small JSON manifest to a Chrome-specific directory)
- When the user clicks the extension button, the extension sends the extracted restaurants as JSON to the CLI process
- The CLI writes the data to `~/.resto-roulette/restaurants.json`
- Fully automatic after setup — no file dialogs, no manual steps

**Trade-off:** Requires the registration step and a small background process or on-demand invocation.

#### Option B: File download (simplest)

The extension uses Chrome's `chrome.downloads` API to save a JSON file directly to a known location.

- One click: user views the list, clicks the extension icon, file is saved
- The CLI reads from the downloaded file path (configurable or defaulting to `~/.resto-roulette/restaurants.json`)
- No registration, no background process

**Trade-off:** Still a manual click per sync, but dramatically less friction than the current 5-step CSV export process. The user might need to configure the download path.

#### Option C: Local HTTP endpoint

The CLI exposes a tiny HTTP server on localhost (e.g. `resto-roulette serve`), and the extension POSTs data to it.

- Fully automatic once both are running
- The CLI can immediately process the new data or cache it

**Trade-off:** Requires the CLI to be running before the extension sends data. Adds a `serve` subcommand and a long-running process, which is a departure from the current run-and-exit design.

### Distribution

**For personal use / development:** Chrome supports [side-loading unpacked extensions](https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world#load-unpacked) via `chrome://extensions` with developer mode enabled. This is free, requires no review process, and works immediately. The extension source lives in the repo and the user loads it directly. This is the recommended starting point.

**For broader distribution:** The [Chrome Web Store](https://developer.chrome.com/docs/webstore/register) charges a one-time $5 developer registration fee. After that, publishing and updates are free. Extensions go through a review process that typically takes a few business days.

### What the extension enables for the CLI

With the extension providing a cached `restaurants.json`, the CLI gains:
- A `sync` cache that doesn't require any Google API calls to populate
- Offline access to the restaurant list (fetch travel times from cache too with `--dry-run`)
- A clean separation: the extension handles list acquisition, the CLI handles everything else

The CLI would add a new source in its resolution chain:

| Priority | Source | Trigger |
|----------|--------|---------|
| 1 | Local file | `--list` flag or `list_path` in config |
| 2 | Extension cache | `~/.resto-roulette/restaurants.json` exists |
| 3 | Error | No list source configured |

---

## Recommendation

**Build a Chrome extension with file-download transfer (Option B) as the initial implementation.** This is the simplest viable approach:

- No Native Messaging registration complexity
- No long-running server process
- One-click export from any shared list page
- Side-load for personal use (free, no review process)
- The CLI changes are minimal — just read from a JSON cache file as a fallback when `--list` isn't provided

If the one-click friction proves annoying, upgrade to Native Messaging (Option A) later — the extension's data extraction logic stays the same, only the transfer mechanism changes.

### Open questions for implementation

- **Extension technology:** Manifest V3 (current Chrome extension standard) with a content script injected on `google.com/maps` pages, or a popup-based approach triggered manually?
- **DOM stability:** How frequently does Google change the Maps list page DOM structure? This determines the expected maintenance burden. Worth investigating by inspecting the current DOM before building.
- **Scope:** Should the extension support extracting from any Maps list view, or specifically from the shared-list URL format?
- **Cache format:** The JSON schema for `restaurants.json` should match the existing `Restaurant` struct (name, address, optional location) for easy deserialization. Should it include metadata like source URL and sync timestamp?
