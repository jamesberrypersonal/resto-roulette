use anyhow::Context;
use clap::Parser;

use config::{Cli, OutputFormat};
use resto_roulette_core::{
    cache::Cache,
    error::AppError,
    picker,
    pipeline::{EnrichOpts, PipelineInputs},
};

mod config;
mod display;
mod tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = config::load(cli).context("failed to load configuration")?;

    let cache_path = dirs::home_dir()
        .ok_or_else(|| AppError::Config("cannot find home directory".into()))?
        .join(".resto-roulette/cache.db");
    let cache = Cache::open(&cache_path, cfg.cache_ttl_hours, cfg.places_cache_ttl_hours)
        .context("failed to open cache")?;

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

    let inputs = PipelineInputs {
        list_path: cfg.list_path.clone(),
        home: cfg.home.clone(),
        api_key: cfg.api_key.clone(),
        dry_run: cfg.dry_run,
        enrich: EnrichOpts {
            open_now: cfg.open_now,
            cuisine_filter: cfg.cuisine.clone(),
            exclude_cuisines: cfg.exclude_cuisines.clone(),
        },
    };

    let buckets = resto_roulette_core::pipeline::run(&inputs, &cache).await?;

    if buckets.near.is_empty() && buckets.mid.is_empty() && buckets.far.is_empty() {
        return Ok(());
    }

    let selection = picker::pick_random(&buckets);

    use std::io::IsTerminal;
    let use_tui = std::io::stdout().is_terminal()
        && cfg.format == OutputFormat::Pretty
        && (cfg.explore || cfg.reroll);

    if use_tui {
        tui::run(&buckets, selection, cfg.explore).context("TUI error")?;
    } else {
        display::render(&selection, cfg.format);
    }

    Ok(())
}
