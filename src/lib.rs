pub mod bucket;
pub mod cache;
pub mod config;
pub mod display;
pub mod error;
pub mod parse;
pub mod picker;
pub mod places;
pub mod routing;
pub mod tui;

use serde::{Deserialize, Serialize};

/// A single restaurant from the input file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Restaurant {
    pub name: String,
    pub address: String,
    /// Populated from GeoJSON coordinates; absent for CSV input.
    pub location: Option<LatLng>,
}

/// WGS-84 coordinate pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

impl Restaurant {
    /// Stable identifier used as the SQLite cache key.
    /// SHA-256 of `"name\x00address"` encoded as lowercase hex.
    pub fn id(&self) -> String {
        crate::cache::sqlite::hash_restaurant(&self.name, &self.address)
    }
}
