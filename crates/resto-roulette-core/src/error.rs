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

    #[error("Missing API key. Set GOOGLE_MAPS_API_KEY, pass --api-key, or add to config.toml.")]
    MissingApiKey,

    #[error("Missing home address. Set RESTO_HOME, pass --home, or add to config.toml.")]
    MissingHome,

    #[error("Missing path to restaurant list. Pass --list, or add to config.toml.")]
    MissingListPath,

    #[error("{0}")]
    Config(String),

    #[error("Google Places API error: {0}\nHint: ensure the Places API (New) is enabled in your Google Cloud project.")]
    PlacesApi(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
