use serde::{Deserialize, Deserializer, Serialize};

// ===== Request types =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeRoutesRequest {
    pub origin: Waypoint,
    pub destination: Waypoint,
    pub travel_mode: ApiTravelMode,
    pub compute_alternative_routes: bool,
    pub route_modifiers: RouteModifiers,
    pub language_code: String,
    pub units: String,
}

#[derive(Serialize)]
pub struct Waypoint {
    pub address: String,
}

#[derive(Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiTravelMode {
    Walk,
    Bicycle,
    Transit,
    Drive,
}

#[derive(Serialize, Default)]
pub struct RouteModifiers {}

// ===== Response types =====

#[derive(Deserialize, Debug)]
pub struct ComputeRoutesResponse {
    pub routes: Option<Vec<Route>>,
}

#[derive(Deserialize, Debug)]
pub struct Route {
    #[serde(deserialize_with = "deserialize_duration_string")]
    pub duration: u32,
}

/// Google Routes API returns duration as a protobuf duration string, e.g. "720s".
fn deserialize_duration_string<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let s = String::deserialize(d)?;
    let trimmed = s.trim_end_matches('s');
    trimmed
        .parse::<u32>()
        .map_err(|e| serde::de::Error::custom(format!("invalid duration '{}': {}", s, e)))
}

// ===== Unified result =====

#[derive(Debug, Clone, Default)]
pub struct TravelTimes {
    pub walk_secs: Option<u32>,
    pub bike_secs: Option<u32>,
    pub transit_secs: Option<u32>,
    pub drive_secs: Option<u32>,
}

impl TravelTimes {
    pub fn is_complete(&self) -> bool {
        self.walk_secs.is_some()
            && self.bike_secs.is_some()
            && self.transit_secs.is_some()
            && self.drive_secs.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TravelMode {
    Walk,
    Bike,
    Transit,
    Drive,
}

impl TravelMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            TravelMode::Walk => "walking",
            TravelMode::Bike => "bike",
            TravelMode::Transit => "transit",
            TravelMode::Drive => "car",
        }
    }

    pub fn db_str(&self) -> &'static str {
        match self {
            TravelMode::Walk => "walk",
            TravelMode::Bike => "bike",
            TravelMode::Transit => "transit",
            TravelMode::Drive => "drive",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_duration_string() {
        let json = r#"{"routes": [{"duration": "720s"}]}"#;
        let resp: ComputeRoutesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.routes.unwrap()[0].duration, 720);
    }

    #[test]
    fn deserialize_empty_routes() {
        let json = r#"{"routes": []}"#;
        let resp: ComputeRoutesResponse = serde_json::from_str(json).unwrap();
        assert!(resp.routes.unwrap().is_empty());
    }
}
