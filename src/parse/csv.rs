use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::Restaurant;

// ===== Simple format: name,address =====

#[derive(Deserialize)]
struct SimpleRow {
    name: String,
    address: String,
}

// ===== Google Maps shared-list export format: Title,Note,URL,Tags,Comment =====

#[derive(Deserialize)]
struct MapsExportRow {
    #[serde(rename = "Title")]
    title: String,
    // remaining columns are present in the file but not needed
}

pub fn parse(path: &Path) -> Result<Vec<Restaurant>> {
    // Peek at the headers to decide which format to use
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| AppError::Parse(e.to_string()))?;

    let headers = rdr
        .headers()
        .map_err(|e| AppError::Parse(e.to_string()))?
        .clone();

    let first = headers.get(0).unwrap_or("").to_lowercase();

    if first == "title" {
        parse_maps_export(rdr)
    } else {
        parse_simple(rdr)
    }
}

fn parse_simple(mut rdr: csv::Reader<std::fs::File>) -> Result<Vec<Restaurant>> {
    let mut restaurants = Vec::new();
    for result in rdr.deserialize::<SimpleRow>() {
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

fn parse_maps_export(mut rdr: csv::Reader<std::fs::File>) -> Result<Vec<Restaurant>> {
    let mut restaurants = Vec::new();
    for result in rdr.deserialize::<MapsExportRow>() {
        match result {
            Ok(row) => {
                let title = row.title.trim().to_string();
                if title.is_empty() {
                    continue;
                }
                // Use the place name as the routing address — the Routes API
                // resolves place names via its own geocoding.
                restaurants.push(Restaurant {
                    address: title.clone(),
                    name: title,
                    location: None,
                });
            }
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
    fn simple_format_parses_expected_count() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        assert_eq!(restaurants.len(), 5);
    }

    #[test]
    fn simple_format_all_locations_are_none() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        assert!(restaurants.iter().all(|r| r.location.is_none()));
    }

    #[test]
    fn simple_format_quoted_address_with_comma() {
        let restaurants = parse(&fixtures_dir().join("sample.csv")).unwrap();
        let r = restaurants
            .iter()
            .find(|r| r.name.contains("Cabane"))
            .unwrap();
        assert!(r.address.contains("Fresnière"), "address={}", r.address);
    }

    #[test]
    fn maps_export_format_skips_blank_rows() {
        let restaurants = parse(&fixtures_dir().join("sample_maps_export.csv")).unwrap();
        assert!(restaurants.iter().all(|r| !r.name.is_empty()));
    }

    #[test]
    fn maps_export_format_uses_title_as_name_and_address() {
        let restaurants = parse(&fixtures_dir().join("sample_maps_export.csv")).unwrap();
        let r = restaurants
            .iter()
            .find(|r| r.name == "Nouveau Palais")
            .unwrap();
        assert_eq!(r.address, "Nouveau Palais");
    }
}
