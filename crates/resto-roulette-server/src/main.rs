use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

use resto_roulette_core::cache::Cache;
use resto_roulette_server::{build_app, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = resto_roulette_server::config::load().context("failed to load server config")?;

    let cache_path = dirs::home_dir()
        .context("cannot find home directory")?
        .join(".resto-roulette/cache.db");
    let cache = Cache::open(&cache_path, 168, 720).context("failed to open cache")?;

    match cache.evict_expired() {
        Ok(n) if n > 0 => tracing::debug!("Evicted {} expired travel-time entries", n),
        Err(e) => tracing::warn!("Cache eviction failed: {}", e),
        _ => {}
    }
    match cache.evict_expired_places() {
        Ok(n) if n > 0 => tracing::debug!("Evicted {} expired place-details entries", n),
        Err(e) => tracing::warn!("Place-details cache eviction failed: {}", e),
        _ => {}
    }

    let state = Arc::new(AppState {
        cache: Arc::new(Mutex::new(cache)),
        cfg: cfg.clone(),
    });

    let app = build_app(Arc::clone(&state));
    let listener = TcpListener::bind(cfg.bind_addr)
        .await
        .with_context(|| format!("failed to bind to {}", cfg.bind_addr))?;
    tracing::info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
