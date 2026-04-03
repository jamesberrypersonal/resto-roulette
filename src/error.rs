use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to parse input file: {0}")]
    Parse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Google API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Cache error: {0}")]
    Cache(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No restaurants found in {bucket} bucket")]
    EmptyBucket { bucket: String },

    #[error("Missing API key. Set GOOGLE_MAPS_API_KEY or pass --api-key.")]
    MissingApiKey,

    #[error("Missing home address. Set RESTO_HOME, pass --home, or add to config.toml.")]
    MissingHome,

    #[error("{0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
