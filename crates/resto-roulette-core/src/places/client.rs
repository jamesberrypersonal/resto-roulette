use reqwest::Client;
use serde_json::json;

use crate::error::{AppError, Result};

use super::models::{PlaceDetails, PlaceDetailsResponse, TextSearchResponse};

const TEXT_SEARCH_URL: &str = "https://places.googleapis.com/v1/places:searchText";
const PLACE_DETAILS_BASE: &str = "https://places.googleapis.com/v1/places";

const TEXT_SEARCH_FIELD_MASK: &str =
    "places.id,places.types,places.regularOpeningHours,places.utcOffsetMinutes,places.displayName";
const PLACE_DETAILS_FIELD_MASK: &str = "types,regularOpeningHours,utcOffsetMinutes";

pub struct PlacesClient {
    inner: Client,
    api_key: String,
}

impl PlacesClient {
    pub fn new(api_key: String) -> Result<Self> {
        let inner = Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(AppError::Api)?;
        Ok(Self { inner, api_key })
    }

    /// Resolve a restaurant's name + address to a PlaceDetails via Text Search.
    /// Returns `None` if the search returns no results.
    pub async fn text_search(&self, name: &str, address: &str) -> Result<Option<PlaceDetails>> {
        let query = format!("{}, {}", name, address);
        let body = json!({
            "textQuery": query,
            "maxResultCount": 1
        });

        let resp = self
            .inner
            .post(TEXT_SEARCH_URL)
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", TEXT_SEARCH_FIELD_MASK)
            .json(&body)
            .send()
            .await
            .map_err(AppError::Api)?;

        let status = resp.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::PlacesApi(
                "403 Forbidden — check that the Places API (New) is enabled for your API key"
                    .into(),
            ));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::PlacesApi(format!(
                "HTTP {} from Text Search: {}",
                status, text
            )));
        }

        let parsed: TextSearchResponse = resp.json().await.map_err(AppError::Api)?;
        Ok(parsed.places.into_iter().next().map(PlaceDetails::from))
    }

    /// Fetch fresh details for a known Place ID. Cheaper than Text Search.
    /// Returns `None` only on unexpected empty responses (rare).
    pub async fn place_details(&self, place_id: &str) -> Result<Option<PlaceDetails>> {
        let url = format!("{}/{}", PLACE_DETAILS_BASE, place_id);

        let resp = self
            .inner
            .get(&url)
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", PLACE_DETAILS_FIELD_MASK)
            .send()
            .await
            .map_err(AppError::Api)?;

        let status = resp.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::PlacesApi(
                "403 Forbidden — check that the Places API (New) is enabled for your API key"
                    .into(),
            ));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::PlacesApi(format!(
                "HTTP {} from Place Details: {}",
                status, text
            )));
        }

        let parsed: PlaceDetailsResponse = resp.json().await.map_err(AppError::Api)?;
        Ok(Some(PlaceDetails::updated_from_details_response(
            place_id.to_string(),
            parsed,
        )))
    }
}
