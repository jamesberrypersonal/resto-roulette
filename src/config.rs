use std::path::{Path, PathBuf};

use clap::Parser;
use serde::Deserialize;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Pretty,
    Json,
}

/// Parsed, fully-resolved configuration ready for use.
pub struct Config {
    pub home: String,
    pub api_key: String,
    pub list_path: PathBuf,
    pub reroll: bool,
    pub format: OutputFormat,
    pub cache_ttl_hours: u64,
    pub dry_run: bool,
}

/// Raw CLI arguments parsed by clap.
#[derive(Debug, Parser)]
#[command(
    name = "resto-roulette",
    about = "Random restaurant picker from your Google Maps saved list"
)]
pub struct Cli {
    /// Home address or lat,lng (env: RESTO_HOME)
    #[arg(short = 'H', long, env = "RESTO_HOME")]
    pub home: Option<String>,

    /// Path to exported list file (CSV or GeoJSON)
    #[arg(short, long, default_value = "saved_places.csv")]
    pub list: PathBuf,

    /// Interactive re-roll mode
    #[arg(short, long, default_value_t = false)]
    pub reroll: bool,

    /// Output format: pretty or json
    #[arg(long)]
    pub format: Option<String>,

    /// Hours to cache travel times
    #[arg(long = "cache-ttl", default_value_t = 168)]
    pub cache_ttl: u64,

    /// Show buckets without API calls (uses cache only)
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Google Maps API key (env: GOOGLE_MAPS_API_KEY)
    #[arg(long, env = "GOOGLE_MAPS_API_KEY")]
    pub api_key: Option<String>,
}

/// Contents of ~/.resto-roulette/config.toml
#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    home: Option<String>,
    api_key: Option<String>,
    cache_ttl_hours: Option<u64>,
    default_format: Option<String>,
}

pub fn load(cli: Cli) -> Result<Config> {
    let file_cfg = match config_path() {
        Some(path) => read_file_config(&path)?,
        None => FileConfig::default(),
    };

    let home = cli.home.or(file_cfg.home).ok_or(AppError::MissingHome)?;

    let api_key = cli
        .api_key
        .or(file_cfg.api_key)
        .ok_or(AppError::MissingApiKey)?;

    let format_str = cli
        .format
        .or(file_cfg.default_format)
        .unwrap_or_else(|| "pretty".into());
    let format = parse_format(&format_str)?;

    let cache_ttl_hours = file_cfg.cache_ttl_hours.unwrap_or(cli.cache_ttl);

    Ok(Config {
        home,
        api_key,
        list_path: cli.list,
        reroll: cli.reroll,
        format,
        cache_ttl_hours,
        dry_run: cli.dry_run,
    })
}

fn parse_format(s: &str) -> Result<OutputFormat> {
    match s.to_lowercase().as_str() {
        "pretty" => Ok(OutputFormat::Pretty),
        "json" => Ok(OutputFormat::Json),
        other => Err(AppError::Config(format!(
            "unknown format '{}': expected 'pretty' or 'json'",
            other
        ))),
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".resto-roulette/config.toml"))
}

fn read_file_config(path: &Path) -> Result<FileConfig> {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents)
            .map_err(|e| AppError::Config(format!("invalid config.toml: {}", e))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileConfig::default()),
        Err(e) => Err(AppError::Io(e)),
    }
}
