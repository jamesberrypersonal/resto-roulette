use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// API response types (deserialized from Places API JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TextSearchResponse {
    #[serde(default)]
    pub places: Vec<PlaceResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceResult {
    pub id: String,
    #[serde(default)]
    pub types: Vec<String>,
    pub regular_opening_hours: Option<ApiOpeningHours>,
    pub utc_offset_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ApiOpeningHours {
    #[serde(default)]
    pub periods: Vec<ApiPeriod>,
}

#[derive(Debug, Deserialize)]
pub struct ApiPeriod {
    pub open: ApiDayTime,
    pub close: Option<ApiDayTime>,
}

#[derive(Debug, Deserialize)]
pub struct ApiDayTime {
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

/// Response from Place Details (New) endpoint — same shape as a single PlaceResult
/// but without the `id` field (we already have it).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceDetailsResponse {
    #[serde(default)]
    pub types: Vec<String>,
    pub regular_opening_hours: Option<ApiOpeningHours>,
    pub utc_offset_minutes: Option<i32>,
}

// ---------------------------------------------------------------------------
// Domain model (used throughout the app, serialized to/from cache)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceDetails {
    pub place_id: String,
    pub types: Vec<String>,
    pub hours: Option<WeeklyHours>,
    pub utc_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyHours {
    pub periods: Vec<HoursPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoursPeriod {
    pub open: DayTime,
    pub close: Option<DayTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayTime {
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

impl From<PlaceResult> for PlaceDetails {
    fn from(r: PlaceResult) -> Self {
        let hours = r.regular_opening_hours.map(|h| WeeklyHours {
            periods: h
                .periods
                .into_iter()
                .map(|p| HoursPeriod {
                    open: DayTime {
                        day: p.open.day,
                        hour: p.open.hour,
                        minute: p.open.minute,
                    },
                    close: p.close.map(|c| DayTime {
                        day: c.day,
                        hour: c.hour,
                        minute: c.minute,
                    }),
                })
                .collect(),
        });
        PlaceDetails {
            place_id: r.id,
            types: r.types,
            hours,
            utc_offset_minutes: r.utc_offset_minutes,
        }
    }
}

impl PlaceDetails {
    /// Merge fresh data from Place Details (New) into an existing PlaceDetails
    /// (preserving the place_id we already know).
    pub fn updated_from_details_response(
        place_id: String,
        resp: PlaceDetailsResponse,
    ) -> PlaceDetails {
        let result = PlaceResult {
            id: place_id,
            types: resp.types,
            regular_opening_hours: resp.regular_opening_hours,
            utc_offset_minutes: resp.utc_offset_minutes,
        };
        PlaceDetails::from(result)
    }
}
