use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::NamedTempFile;
use tower::ServiceExt;

use resto_roulette_core::cache::Cache;
use resto_roulette_server::config::ServerConfig;
use resto_roulette_server::{build_app, AppState};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("resto-roulette-core/tests/fixtures")
        .join(name)
}

fn make_state(auth_token: &str) -> Arc<AppState> {
    let db_file = NamedTempFile::new().unwrap();
    let cache = Cache::open(db_file.path(), 168, 720).unwrap();

    Arc::new(AppState {
        cache: Arc::new(tokio::sync::Mutex::new(cache)),
        cfg: ServerConfig {
            home: "123 Main St, Montreal, QC".into(),
            list_path: fixture_path("sample.geojson"),
            api_key: "test-key".into(),
            auth_token: auth_token.into(),
            bind_addr: "127.0.0.1:0".parse().unwrap(),
        },
    })
}

#[tokio::test]
async fn healthz_returns_200_without_auth() {
    let app = build_app(make_state("secret"));
    let req = Request::builder()
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn trmnl_without_token_returns_401() {
    let app = build_app(make_state("secret"));
    let req = Request::builder()
        .uri("/trmnl")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn trmnl_with_wrong_token_returns_401() {
    let app = build_app(make_state("secret"));
    let req = Request::builder()
        .uri("/trmnl?token=wrong___")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn trmnl_with_correct_query_token_returns_200() {
    let app = build_app(make_state("mysecret"));
    let req = Request::builder()
        .uri("/trmnl?token=mysecret")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("generated_at").is_some());
    assert!(json.get("near").is_some());
    assert!(json.get("mid").is_some());
    assert!(json.get("far").is_some());
}

#[tokio::test]
async fn trmnl_with_correct_header_token_returns_200() {
    let app = build_app(make_state("mysecret"));
    let req = Request::builder()
        .uri("/trmnl")
        .header("X-Auth-Token", "mysecret")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
