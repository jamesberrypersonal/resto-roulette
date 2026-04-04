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
#[derive(Debug)]
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
    #[arg(short, long)]
    pub list: Option<PathBuf>,

    /// Pick once and exit without prompting to re-roll
    #[arg(short, long, default_value_t = false)]
    pub one_shot: bool,

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
    list_path: Option<PathBuf>,
    api_key: Option<String>,
    cache_ttl_hours: Option<u64>,
    default_format: Option<String>,
}

pub fn load(cli: Cli) -> Result<Config> {
    let file_cfg = match config_path() {
        Some(path) => read_file_config(&path)?,
        None => FileConfig::default(),
    };
    resolve(cli, file_cfg)
}

fn resolve(cli: Cli, file_cfg: FileConfig) -> Result<Config> {
    let home = cli.home.or(file_cfg.home).ok_or(AppError::MissingHome)?;

    let list_path = cli
        .list
        .or(file_cfg.list_path)
        .ok_or(AppError::MissingListPath)?;

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
        list_path,
        reroll: !cli.one_shot,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn full_cli() -> Cli {
        Cli {
            home: Some("456 Oak Ave".into()),
            list: Some(PathBuf::from("places.csv")),
            one_shot: false,
            format: None,
            cache_ttl: 168,
            dry_run: false,
            api_key: Some("test-key".into()),
        }
    }

    // --- parse_format ---

    #[test]
    fn parse_format_pretty() {
        assert_eq!(parse_format("pretty").unwrap(), OutputFormat::Pretty);
    }

    #[test]
    fn parse_format_json() {
        assert_eq!(parse_format("json").unwrap(), OutputFormat::Json);
    }

    #[test]
    fn parse_format_case_insensitive() {
        assert_eq!(parse_format("Pretty").unwrap(), OutputFormat::Pretty);
        assert_eq!(parse_format("JSON").unwrap(), OutputFormat::Json);
    }

    #[test]
    fn parse_format_unknown_returns_error() {
        assert!(matches!(
            parse_format("csv").unwrap_err(),
            AppError::Config(_)
        ));
    }

    // --- read_file_config ---

    #[test]
    fn read_file_config_missing_file_returns_default() {
        let cfg = read_file_config(Path::new("/nonexistent/path/config.toml")).unwrap();
        assert!(cfg.home.is_none());
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn read_file_config_valid_toml() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "home = \"123 Main St\"\napi_key = \"abc123\"").unwrap();
        let cfg = read_file_config(f.path()).unwrap();
        assert_eq!(cfg.home.as_deref(), Some("123 Main St"));
        assert_eq!(cfg.api_key.as_deref(), Some("abc123"));
    }

    #[test]
    fn read_file_config_invalid_toml() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "not = [valid").unwrap();
        assert!(matches!(
            read_file_config(f.path()).unwrap_err(),
            AppError::Config(_)
        ));
    }

    // --- resolve (load logic, isolated from real config file) ---

    #[test]
    fn resolve_all_cli_values() {
        let cfg = resolve(full_cli(), FileConfig::default()).unwrap();
        assert_eq!(cfg.home, "456 Oak Ave");
        assert_eq!(cfg.api_key, "test-key");
    }

    #[test]
    fn resolve_missing_home_returns_error() {
        let mut cli = full_cli();
        cli.home = None;
        assert!(matches!(
            resolve(cli, FileConfig::default()).unwrap_err(),
            AppError::MissingHome
        ));
    }

    #[test]
    fn resolve_missing_api_key_returns_error() {
        let mut cli = full_cli();
        cli.api_key = None;
        assert!(matches!(
            resolve(cli, FileConfig::default()).unwrap_err(),
            AppError::MissingApiKey
        ));
    }

    #[test]
    fn resolve_missing_list_returns_error() {
        let mut cli = full_cli();
        cli.list = None;
        assert!(matches!(
            resolve(cli, FileConfig::default()).unwrap_err(),
            AppError::MissingListPath
        ));
    }

    #[test]
    fn resolve_file_config_fills_missing_cli_values() {
        let mut cli = full_cli();
        cli.home = None;
        cli.api_key = None;
        let file_cfg = FileConfig {
            home: Some("789 Pine Rd".into()),
            api_key: Some("file-key".into()),
            ..FileConfig::default()
        };
        let cfg = resolve(cli, file_cfg).unwrap();
        assert_eq!(cfg.home, "789 Pine Rd");
        assert_eq!(cfg.api_key, "file-key");
    }

    #[test]
    fn resolve_cli_takes_precedence_over_file_config() {
        let file_cfg = FileConfig {
            home: Some("file home".into()),
            api_key: Some("file-key".into()),
            ..FileConfig::default()
        };
        let cfg = resolve(full_cli(), file_cfg).unwrap();
        assert_eq!(cfg.home, "456 Oak Ave");
        assert_eq!(cfg.api_key, "test-key");
    }

    #[test]
    fn resolve_default_format_is_pretty() {
        let cfg = resolve(full_cli(), FileConfig::default()).unwrap();
        assert_eq!(cfg.format, OutputFormat::Pretty);
    }

    #[test]
    fn resolve_json_format() {
        let mut cli = full_cli();
        cli.format = Some("json".into());
        let cfg = resolve(cli, FileConfig::default()).unwrap();
        assert_eq!(cfg.format, OutputFormat::Json);
    }

    #[test]
    fn resolve_invalid_format_returns_error() {
        let mut cli = full_cli();
        cli.format = Some("xml".into());
        assert!(matches!(
            resolve(cli, FileConfig::default()).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn resolve_cache_ttl_file_overrides_cli() {
        let file_cfg = FileConfig {
            cache_ttl_hours: Some(24),
            ..FileConfig::default()
        };
        let cfg = resolve(full_cli(), file_cfg).unwrap();
        assert_eq!(cfg.cache_ttl_hours, 24);
    }

    #[test]
    fn resolve_cache_ttl_falls_back_to_cli() {
        let mut cli = full_cli();
        cli.cache_ttl = 48;
        let cfg = resolve(cli, FileConfig::default()).unwrap();
        assert_eq!(cfg.cache_ttl_hours, 48);
    }

    #[test]
    fn resolve_dry_run_flag() {
        let mut cli = full_cli();
        cli.dry_run = true;
        assert!(resolve(cli, FileConfig::default()).unwrap().dry_run);
    }

    #[test]
    fn resolve_reroll_default() {
        let cfg = resolve(full_cli(), FileConfig::default()).unwrap();
        assert!(cfg.reroll);
    }

    #[test]
    fn resolve_one_shot_flag() {
        let mut cli = full_cli();
        cli.one_shot = true;
        assert!(!resolve(cli, FileConfig::default()).unwrap().reroll);
    }
}
