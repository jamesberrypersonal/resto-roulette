use std::collections::HashMap;
use std::io::Write as IoWrite;

use anyhow::Context;
use clap::Parser;
use futures::StreamExt;

use resto_roulette::{
    bucket,
    cache::Cache,
    cache::sqlite::hash_home,
    config::{self, Cli},
    display,
    error::AppError,
    parse,
    picker,
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
    let cache = Cache::open(&cache_path, cfg.cache_ttl_hours)
        .context("failed to open cache")?;

    // Evict stale entries at startup (best-effort)
    match cache.evict_expired() {
        Ok(n) if n > 0 => tracing::debug!("Evicted {} expired cache entries", n),
        Err(e) => tracing::warn!("Cache eviction failed: {}", e),
        _ => {}
    }

    let home_id = hash_home(&cfg.home);
    let client = RoutingClient::new(cfg.api_key.clone()).context("failed to build HTTP client")?;

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
                let cached = cache.get(&rid, home_id, dry_run)
                    .unwrap_or_default();

                let times = if cached.is_complete() {
                    tracing::debug!("Cache hit for '{}'", restaurant.name);
                    cached
                } else if dry_run {
                    tracing::debug!("Dry run: using partial/empty cache for '{}'", restaurant.name);
                    cached
                } else {
                    tracing::debug!("Fetching travel times for '{}'", restaurant.name);
                    let fetched = client
                        .get_travel_times(home, &restaurant.address, restaurant.location)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to fetch times for '{}': {}", restaurant.name, e);
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
