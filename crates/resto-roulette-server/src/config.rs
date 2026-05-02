use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use resto_roulette_core::error::AppError;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub home: String,
    pub list_path: PathBuf,
    pub api_key: String,
    pub auth_token: String,
    pub bind_addr: SocketAddr,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    home: Option<String>,
    list_path: Option<PathBuf>,
    api_key: Option<String>,
    auth_token: Option<String>,
    bind_addr: Option<String>,
}

pub fn load() -> Result<ServerConfig, AppError> {
    let file_cfg = match config_path() {
        Some(path) => read_file_config(&path)?,
        None => FileConfig::default(),
    };
    let env_api_key = std::env::var("GOOGLE_MAPS_API_KEY").ok();
    let env_auth_token = std::env::var("RESTO_AUTH_TOKEN").ok();
    resolve(file_cfg, env_api_key, env_auth_token)
}

fn resolve(
    file: FileConfig,
    env_api_key: Option<String>,
    env_auth_token: Option<String>,
) -> Result<ServerConfig, AppError> {
    let home = file
        .home
        .ok_or_else(|| AppError::Config("missing required field 'home' in server.toml".into()))?;
    let list_path = file.list_path.ok_or_else(|| {
        AppError::Config("missing required field 'list_path' in server.toml".into())
    })?;
    let api_key = env_api_key.or(file.api_key).ok_or_else(|| {
        AppError::Config(
            "missing api_key: set GOOGLE_MAPS_API_KEY or api_key in server.toml".into(),
        )
    })?;
    let auth_token = env_auth_token.or(file.auth_token).ok_or_else(|| {
        AppError::Config(
            "missing auth_token: set RESTO_AUTH_TOKEN or auth_token in server.toml".into(),
        )
    })?;
    let bind_addr = file
        .bind_addr
        .as_deref()
        .unwrap_or("127.0.0.1:8080")
        .parse::<SocketAddr>()
        .map_err(|e| AppError::Config(format!("invalid bind_addr: {}", e)))?;

    Ok(ServerConfig {
        home,
        list_path,
        api_key,
        auth_token,
        bind_addr,
    })
}

fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".resto-roulette/server.toml"))
}

fn read_file_config(path: &Path) -> Result<FileConfig, AppError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents)
            .map_err(|e| AppError::Config(format!("invalid server.toml: {}", e))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(FileConfig::default()),
        Err(e) => Err(AppError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_file() -> FileConfig {
        FileConfig {
            home: Some("123 Main St".into()),
            list_path: Some(PathBuf::from("/tmp/list.geojson")),
            api_key: Some("file-key".into()),
            auth_token: Some("file-token".into()),
            bind_addr: None,
        }
    }

    #[test]
    fn resolve_all_from_file() {
        let cfg = resolve(full_file(), None, None).unwrap();
        assert_eq!(cfg.home, "123 Main St");
        assert_eq!(cfg.api_key, "file-key");
        assert_eq!(cfg.auth_token, "file-token");
        assert_eq!(cfg.bind_addr, "127.0.0.1:8080".parse().unwrap());
    }

    #[test]
    fn env_overrides_file_api_key() {
        let cfg = resolve(full_file(), Some("env-key".into()), None).unwrap();
        assert_eq!(cfg.api_key, "env-key");
    }

    #[test]
    fn env_overrides_file_auth_token() {
        let cfg = resolve(full_file(), None, Some("env-token".into())).unwrap();
        assert_eq!(cfg.auth_token, "env-token");
    }

    #[test]
    fn missing_home_errors() {
        let mut f = full_file();
        f.home = None;
        assert!(matches!(
            resolve(f, None, None).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn missing_list_path_errors() {
        let mut f = full_file();
        f.list_path = None;
        assert!(matches!(
            resolve(f, None, None).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn missing_api_key_errors() {
        let mut f = full_file();
        f.api_key = None;
        assert!(matches!(
            resolve(f, None, None).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn missing_auth_token_errors() {
        let mut f = full_file();
        f.auth_token = None;
        assert!(matches!(
            resolve(f, None, None).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn custom_bind_addr() {
        let mut f = full_file();
        f.bind_addr = Some("0.0.0.0:9090".into());
        let cfg = resolve(f, None, None).unwrap();
        assert_eq!(cfg.bind_addr, "0.0.0.0:9090".parse().unwrap());
    }

    #[test]
    fn invalid_bind_addr_errors() {
        let mut f = full_file();
        f.bind_addr = Some("not-an-addr".into());
        assert!(matches!(
            resolve(f, None, None).unwrap_err(),
            AppError::Config(_)
        ));
    }

    #[test]
    fn read_file_config_missing_file_returns_default() {
        let cfg = read_file_config(Path::new("/nonexistent/server.toml")).unwrap();
        assert!(cfg.home.is_none());
    }
}
