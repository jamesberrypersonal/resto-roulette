use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::Restaurant;

#[derive(Deserialize)]
struct CsvRow {
    name: String,
    address: String,
}

pub fn parse(path: &Path) -> Result<Vec<Restaurant>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| AppError::Parse(e.to_string()))?;

    let mut restaurants = Vec::new();

    for result in rdr.deserialize::<CsvRow>() {
        match result {
            Ok(row) => restaurants.push(Restaurant {
                name: row.name,
                address: row.address,
                location: None,
            }),
            Err(e) => tracing::warn!("Skipping malformed CSV row: {}", e),
        }
    }

    Ok(restaurants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    #[test]
    fn parses_expected_count() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        assert_eq!(restaurants.len(), 5);
    }

    #[test]
    fn all_locations_are_none() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        assert!(restaurants.iter().all(|r| r.location.is_none()));
    }

    #[test]
    fn quoted_address_with_comma() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        let r = restaurants.iter().find(|r| r.name.contains("Cabane")).unwrap();
        assert!(r.address.contains("Fresnière"), "address={}", r.address);
    }
}
