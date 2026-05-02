use std::sync::Arc;

use axum::{extract::State, http::StatusCode, middleware, response::Json, routing::get, Router};
use chrono::Utc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;

use resto_roulette_core::{
    cache::Cache,
    error::AppError,
    picker,
    pipeline::{EnrichOpts, PipelineInputs},
};

use crate::config::ServerConfig;
use crate::render;

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<Mutex<Cache>>,
    pub cfg: ServerConfig,
}

pub fn build_app(state: Arc<AppState>) -> Router {
    let auth_token = Arc::new(state.cfg.auth_token.clone());

    // Build the authenticated sub-router with explicit state type, then resolve it.
    let authed: Router = Router::<Arc<AppState>>::new()
        .route("/trmnl", get(handle_trmnl))
        .route_layer(middleware::from_fn_with_state(
            auth_token,
            crate::auth::require_token,
        ))
        .with_state(Arc::clone(&state));

    Router::new()
        .route("/healthz", get(handle_healthz))
        .merge(authed)
        .layer(TraceLayer::new_for_http())
}

async fn handle_healthz() -> &'static str {
    "ok"
}

async fn handle_trmnl(
    State(state): State<Arc<AppState>>,
) -> Result<Json<render::TrmnlResponse>, StatusCode> {
    let inputs = PipelineInputs {
        list_path: state.cfg.list_path.clone(),
        home: state.cfg.home.clone(),
        api_key: state.cfg.api_key.clone(),
        dry_run: false,
        enrich: EnrichOpts::server_v1(),
    };

    // Mutex held across the pipeline await — documented single-tenant v1 trade-off.
    let cache = state.cache.lock().await;
    let buckets = resto_roulette_core::pipeline::run(&inputs, &cache)
        .await
        .map_err(|e: AppError| {
            tracing::error!("pipeline error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    drop(cache);

    let selection = picker::pick_random(&buckets);
    let response = render::from_selection(&selection, Utc::now());
    Ok(Json(response))
}
