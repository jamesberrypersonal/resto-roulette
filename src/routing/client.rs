use std::time::Duration;

use reqwest::Client;

use crate::error::Result;

use super::models::{
    ApiTravelMode, ComputeRoutesRequest, ComputeRoutesResponse, RouteModifiers, TravelMode,
    TravelTimes, Waypoint,
};

const ROUTES_API_URL: &str = "https://routes.googleapis.com/directions/v2:computeRoutes";
const LANGUAGE_CODE: &str = "en-US";
const UNITS: &str = "METRIC";

pub struct RoutingClient {
    client: Client,
    api_key: String,
}

impl RoutingClient {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
        Ok(Self { client, api_key })
    }

    /// Fetch travel times for all four modes between home and restaurant.
    /// Individual mode failures are non-fatal (logged as warnings, result in None).
    pub async fn get_travel_times(&self, home: &str, destination: &str) -> Result<TravelTimes> {
        let (walk, bike, transit, drive) = tokio::join!(
            self.fetch_one_mode(home, destination, ApiTravelMode::Walk),
            self.fetch_one_mode(home, destination, ApiTravelMode::Bicycle),
            self.fetch_one_mode(home, destination, ApiTravelMode::Transit),
            self.fetch_one_mode(home, destination, ApiTravelMode::Drive),
        );

        Ok(TravelTimes {
            walk_secs: flatten_result(walk, TravelMode::Walk),
            bike_secs: flatten_result(bike, TravelMode::Bike),
            transit_secs: flatten_result(transit, TravelMode::Transit),
            drive_secs: flatten_result(drive, TravelMode::Drive),
        })
    }

    async fn fetch_one_mode(
        &self,
        origin: &str,
        destination: &str,
        mode: ApiTravelMode,
    ) -> Result<Option<u32>> {
        let body = ComputeRoutesRequest {
            origin: Waypoint {
                address: origin.to_string(),
            },
            destination: Waypoint {
                address: destination.to_string(),
            },
            travel_mode: mode,
            compute_alternative_routes: false,
            route_modifiers: RouteModifiers::default(),
            language_code: LANGUAGE_CODE.to_string(),
            units: UNITS.to_string(),
        };

        let resp = self
            .client
            .post(ROUTES_API_URL)
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", "routes.duration")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("Routes API {:?} returned {}: {}", mode, status, text);
            return Ok(None);
        }

        let parsed: ComputeRoutesResponse = resp.json().await?;
        let secs = parsed
            .routes
            .and_then(|routes| routes.into_iter().next())
            .map(|r| r.duration);

        Ok(secs)
    }
}

fn flatten_result(result: Result<Option<u32>>, mode: TravelMode) -> Option<u32> {
    match result {
        Ok(val) => val,
        Err(e) => {
            tracing::warn!("Failed to fetch {} travel time: {}", mode.display_name(), e);
            None
        }
    }
}
