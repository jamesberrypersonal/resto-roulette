use std::path::Path;

use crate::{
    error::{AppError, Result},
    Restaurant,
};

pub mod csv;
pub mod geojson;

pub fn parse_file(path: &Path) -> Result<Vec<Restaurant>> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("geojson") | Some("json") => geojson::parse(path),
        Some("csv") => csv::parse(path),
        other => Err(AppError::Parse(format!(
            "unsupported file extension: {:?}",
            other
        ))),
    }
}
