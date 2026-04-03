use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, Result};
use crate::{LatLng, Restaurant};

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    properties: Option<Properties>,
    geometry: Option<Geometry>,
}

#[derive(Deserialize)]
struct Properties {
    name: Option<String>,
    address: Option<String>,
}

#[derive(Deserialize)]
struct Geometry {
    #[serde(rename = "type")]
    geometry_type: String,
    /// GeoJSON spec: coordinates are [longitude, latitude]
    coordinates: Option<[f64; 2]>,
}

pub fn parse(path: &Path) -> Result<Vec<Restaurant>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let collection: FeatureCollection = serde_json::from_reader(reader)
        .map_err(|e| AppError::Parse(format!("invalid GeoJSON: {}", e)))?;

    let mut restaurants = Vec::new();

    for feature in collection.features {
        let props = match feature.properties {
            Some(p) => p,
            None => {
                tracing::warn!("Skipping GeoJSON feature with no properties");
                continue;
            }
        };

        let name = match props.name {
            Some(n) if !n.is_empty() => n,
            _ => {
                tracing::warn!("Skipping GeoJSON feature with missing name");
                continue;
            }
        };

        let address = props.address.unwrap_or_default();

        // GeoJSON coordinates are [longitude, latitude] — swap to lat/lng
        let location = feature.geometry.and_then(|g| {
            if g.geometry_type == "Point" {
                g.coordinates.map(|coords| LatLng {
                    lat: coords[1],
                    lng: coords[0],
                })
            } else {
                None
            }
        });

        restaurants.push(Restaurant { name, address, location });
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
        let restaurants = parse(&fixtures_dir().join("sample.geojson")).unwrap();
        assert_eq!(restaurants.len(), 5);
    }

    #[test]
    fn coordinates_are_lat_lng_order() {
        let restaurants = parse(&fixtures_dir().join("sample.geojson")).unwrap();
        let ha = restaurants.iter().find(|r| r.name == "Hà").unwrap();
        let loc = ha.location.unwrap();
        // lat ~45.5, lng ~-73.5
        assert!((loc.lat - 45.509).abs() < 0.01, "lat={}", loc.lat);
        assert!((loc.lng - (-73.5605)).abs() < 0.01, "lng={}", loc.lng);
    }

    #[test]
    fn null_geometry_yields_none_location() {
        let restaurants = parse(&fixtures_dir().join("sample.geojson")).unwrap();
        let r = restaurants.iter().find(|r| r.name == "Restaurant Sans Adresse").unwrap();
        assert!(r.location.is_none());
    }
}
