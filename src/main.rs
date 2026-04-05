use std::collections::HashMap;
use std::io::Write as IoWrite;

use anyhow::Context;
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;

use resto_roulette::{
    bucket,
    cache::sqlite::hash_home,
    cache::Cache,
    config::{self, Cli},
    display,
    error::AppError,
    parse, picker,
    places::{self, PlacesClient},
    routing::RoutingClient,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = config::load(cli).context("failed to load configuration")?;

    // Parse input file
    let restaurants = parse::parse_file(&cfg.list_path)
        .with_context(|| format!("failed to parse {:?}", cfg.list_path))?;
    tracing::info!("Loaded {} restaurants", restaurants.len());

    if restaurants.is_empty() {
        eprintln!("No restaurants found in {:?}", cfg.list_path);
        return Ok(());
    }

    // Open cache
    let cache_path = dirs::home_dir()
        .ok_or_else(|| AppError::Config("cannot find home directory".into()))?
        .join(".resto-roulette/cache.db");
    let cache = Cache::open(&cache_path, cfg.cache_ttl_hours, cfg.places_cache_ttl_hours)
        .context("failed to open cache")?;

    // Evict stale entries at startup (best-effort)
    match cache.evict_expired() {
        Ok(n) if n > 0 => tracing::debug!("Evicted {} expired cache entries", n),
        Err(e) => tracing::warn!("Cache eviction failed: {}", e),
        _ => {}
    }
    match cache.evict_expired_places() {
        Ok(n) if n > 0 => tracing::debug!("Evicted {} expired place_details cache entries", n),
        Err(e) => tracing::warn!("Place details cache eviction failed: {}", e),
        _ => {}
    }

    let home_id = hash_home(&cfg.home);
    let client = RoutingClient::new(cfg.api_key.clone()).context("failed to build HTTP client")?;

    // Enrich with Places API (lazy: only when --open-now is active)
    let place_details_map: HashMap<String, places::PlaceDetails> = if cfg.open_now {
        let places_client =
            PlacesClient::new(cfg.api_key.clone()).context("failed to build Places API client")?;

        futures::stream::iter(restaurants.iter())
            .map(|restaurant| {
                let rid = restaurant.id();
                let cache = &cache;
                let places_client = &places_client;
                let dry_run = cfg.dry_run;
                async move {
                    let cached = cache.get_place(&rid, dry_run).unwrap_or(None);

                    let details = match cached {
                        // Fresh hit — use as-is
                        Some((details, true)) => {
                            tracing::debug!("Place cache hit (fresh) for '{}'", restaurant.name);
                            Some(details)
                        }
                        // Stale hit in dry-run — use stale data
                        Some((details, false)) if dry_run => {
                            tracing::debug!(
                                "Dry run: using stale place cache for '{}'",
                                restaurant.name
                            );
                            Some(details)
                        }
                        // Stale hit — refresh via cheaper Place Details API
                        Some((stale, false)) => {
                            tracing::debug!(
                                "Place cache stale for '{}', refreshing via Place Details",
                                restaurant.name
                            );
                            let refreshed = places_client
                                .place_details(&stale.place_id)
                                .await
                                .unwrap_or_else(|e| {
                                    tracing::warn!(
                                        "Failed to refresh place details for '{}': {}",
                                        restaurant.name,
                                        e
                                    );
                                    None
                                });
                            let result = refreshed.or(Some(stale));
                            if let Some(ref d) = result {
                                if let Err(e) = cache.put_place(&rid, d) {
                                    tracing::warn!(
                                        "Failed to cache place details for '{}': {}",
                                        restaurant.name,
                                        e
                                    );
                                }
                            }
                            result
                        }
                        // Miss in dry-run — fail-open (can't determine open status)
                        None if dry_run => {
                            tracing::debug!(
                                "Dry run: no place cache for '{}', keeping (fail-open)",
                                restaurant.name
                            );
                            None
                        }
                        // Miss — resolve via Text Search
                        None => {
                            tracing::debug!("Fetching place details for '{}'", restaurant.name);
                            let fetched = places_client
                                .text_search(&restaurant.name, &restaurant.address)
                                .await
                                .unwrap_or_else(|e| {
                                    tracing::warn!(
                                        "Failed to fetch place details for '{}': {}",
                                        restaurant.name,
                                        e
                                    );
                                    None
                                });
                            if let Some(ref d) = fetched {
                                if let Err(e) = cache.put_place(&rid, d) {
                                    tracing::warn!(
                                        "Failed to cache place details for '{}': {}",
                                        restaurant.name,
                                        e
                                    );
                                }
                            }
                            fetched
                        }
                    };

                    (rid, details)
                }
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|(rid, opt)| opt.map(|d| (rid, d)))
            .collect()
    } else {
        HashMap::new()
    };

    // Filter closed restaurants (only when --open-now)
    let restaurants = if cfg.open_now {
        let now = Utc::now();
        let original_len = restaurants.len();
        let open: Vec<_> = restaurants
            .into_iter()
            .filter(|r| {
                match place_details_map.get(&r.id()) {
                    Some(d) => match (&d.hours, d.utc_offset_minutes) {
                        (Some(h), Some(offset)) => places::hours::is_open_at(h, offset, now),
                        _ => true, // no hours or offset = fail-open
                    },
                    None => true, // no details = fail-open
                }
            })
            .collect();
        let skipped = original_len - open.len();
        if skipped > 0 {
            eprintln!(
                "Skipped {} closed restaurant{}.",
                skipped,
                if skipped == 1 { "" } else { "s" }
            );
        }
        if open.is_empty() {
            eprintln!("No open restaurants found. Try without --open-now.");
            return Ok(());
        }
        open
    } else {
        restaurants
    };

    // Fetch travel times: check cache first, call API for misses (concurrently)
    let all_times: HashMap<String, _> = futures::stream::iter(restaurants.iter())
        .map(|restaurant| {
            let rid = restaurant.id();
            let home_id = &home_id;
            let cache = &cache;
            let client = &client;
            let home = &cfg.home;
            let dry_run = cfg.dry_run;
            async move {
                let cached = cache.get(&rid, home_id, dry_run).unwrap_or_default();

                let times = if cached.is_complete() {
                    tracing::debug!("Cache hit for '{}'", restaurant.name);
                    cached
                } else if dry_run {
                    tracing::debug!(
                        "Dry run: using partial/empty cache for '{}'",
                        restaurant.name
                    );
                    cached
                } else {
                    tracing::debug!("Fetching travel times for '{}'", restaurant.name);
                    let fetched = client
                        .get_travel_times(home, &restaurant.address)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                "Failed to fetch times for '{}': {}",
                                restaurant.name,
                                e
                            );
                            Default::default()
                        });
                    if let Err(e) = cache.put(&rid, home_id, &fetched) {
                        tracing::warn!("Failed to cache times for '{}': {}", restaurant.name, e);
                    }
                    fetched
                };

                (rid, times)
            }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    // Bucket and pick
    let buckets = bucket::assign(&restaurants, &all_times);

    if buckets.near.is_empty() && buckets.mid.is_empty() && buckets.far.is_empty() {
        eprintln!("No restaurants could be bucketed (try running without --dry-run to fetch travel times).");
        return Ok(());
    }

    let selection = picker::pick_random(&buckets);
    display::render(&selection, cfg.format);

    // Re-roll loop
    if cfg.reroll {
        loop {
            print!("Re-roll? [y/N] ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                let selection = picker::pick_random(&buckets);
                display::render(&selection, cfg.format);
            } else {
                break;
            }
        }
    }

    Ok(())
}
