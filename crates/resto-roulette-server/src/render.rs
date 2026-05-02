use chrono::{DateTime, Utc};
use serde::Serialize;

use resto_roulette_core::bucket::BucketEntry;
use resto_roulette_core::picker::Selection;

#[derive(Debug, Serialize)]
pub struct TrmnlResponse {
    pub generated_at: DateTime<Utc>,
    pub near: Option<TrmnlPick>,
    pub mid: Option<TrmnlPick>,
    pub far: Option<TrmnlPick>,
}

#[derive(Debug, Serialize)]
pub struct TrmnlPick {
    pub name: String,
    pub address: String,
    pub duration_minutes: u32,
    pub mode: &'static str,
    pub cuisine: Option<String>,
}

pub fn from_selection(sel: &Selection, now: DateTime<Utc>) -> TrmnlResponse {
    TrmnlResponse {
        generated_at: now,
        near: sel.near.as_ref().map(entry_to_pick),
        mid: sel.mid.as_ref().map(entry_to_pick),
        far: sel.far.as_ref().map(entry_to_pick),
    }
}

fn entry_to_pick(e: &BucketEntry) -> TrmnlPick {
    TrmnlPick {
        name: e.restaurant.name.clone(),
        address: e.restaurant.address.clone(),
        duration_minutes: e.best_secs / 60,
        mode: e.best_mode.db_str(),
        cuisine: e.cuisines.first().cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use resto_roulette_core::bucket::{Bucket, BucketEntry};
    use resto_roulette_core::picker::Selection;
    use resto_roulette_core::routing::models::TravelMode;
    use resto_roulette_core::Restaurant;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 8, 0, 0).unwrap()
    }

    fn make_entry(
        name: &str,
        bucket: Bucket,
        secs: u32,
        mode: TravelMode,
        cuisines: Vec<String>,
    ) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: "123 Rue Main".into(),
                location: None,
            },
            bucket,
            best_secs: secs,
            best_mode: mode,
            cuisines,
        }
    }

    #[test]
    fn full_house_renders_three_picks() {
        let sel = Selection {
            near: Some(make_entry(
                "Near Place",
                Bucket::Near,
                600,
                TravelMode::Walk,
                vec!["vietnamese".into()],
            )),
            mid: Some(make_entry(
                "Mid Place",
                Bucket::Mid,
                1200,
                TravelMode::Bike,
                vec![],
            )),
            far: Some(make_entry(
                "Far Place",
                Bucket::Far,
                2400,
                TravelMode::Drive,
                vec!["italian".into()],
            )),
        };
        let resp = from_selection(&sel, fixed_now());

        let near = resp.near.unwrap();
        assert_eq!(near.name, "Near Place");
        assert_eq!(near.duration_minutes, 10);
        assert_eq!(near.mode, "walk");
        assert_eq!(near.cuisine.as_deref(), Some("vietnamese"));

        let mid = resp.mid.unwrap();
        assert_eq!(mid.mode, "bike");
        assert!(mid.cuisine.is_none());

        let far = resp.far.unwrap();
        assert_eq!(far.mode, "drive");
        assert_eq!(far.cuisine.as_deref(), Some("italian"));
    }

    #[test]
    fn empty_bucket_renders_null() {
        let sel = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let resp = from_selection(&sel, fixed_now());
        assert!(resp.near.is_none());
        assert!(resp.mid.is_none());
        assert!(resp.far.is_none());

        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["near"].is_null());
    }

    #[test]
    fn no_cuisines_gives_null_cuisine() {
        let sel = Selection {
            near: Some(make_entry(
                "Place",
                Bucket::Near,
                300,
                TravelMode::Walk,
                vec![],
            )),
            mid: None,
            far: None,
        };
        let resp = from_selection(&sel, fixed_now());
        assert!(resp.near.unwrap().cuisine.is_none());
    }

    #[test]
    fn mode_strings_for_all_travel_modes() {
        let modes = [
            (TravelMode::Walk, "walk"),
            (TravelMode::Bike, "bike"),
            (TravelMode::Transit, "transit"),
            (TravelMode::Drive, "drive"),
        ];
        for (mode, expected) in modes {
            let e = make_entry("X", Bucket::Near, 600, mode, vec![]);
            assert_eq!(entry_to_pick(&e).mode, expected);
        }
    }

    #[test]
    fn generated_at_matches_injected_time() {
        let sel = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let now = fixed_now();
        let resp = from_selection(&sel, now);
        assert_eq!(resp.generated_at, now);
    }
}
