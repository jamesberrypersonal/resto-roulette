use std::collections::HashMap;

use crate::routing::models::{TravelMode, TravelTimes};
use crate::Restaurant;

const NEAR_SECS: u32 = 15 * 60; // 900
const MID_SECS: u32 = 30 * 60;  // 1800
const FAR_SECS: u32 = 60 * 60;  // 3600

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Bucket {
    Near,
    Mid,
    Far,
}

impl Bucket {
    pub fn label(&self) -> &'static str {
        match self {
            Bucket::Near => "≤15 min",
            Bucket::Mid => "15–30 min",
            Bucket::Far => "30–60 min",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Bucket::Near => "🚶",
            Bucket::Mid => "🚲",
            Bucket::Far => "🚗",
        }
    }

    pub fn mode_description(&self) -> &'static str {
        match self {
            Bucket::Near => "Walk/Bike/Transit",
            Bucket::Mid => "Bike/Transit",
            Bucket::Far => "Bike/Transit/Car",
        }
    }
}

/// A restaurant paired with its winning travel time for a given bucket.
#[derive(Debug, Clone)]
pub struct BucketEntry {
    pub restaurant: Restaurant,
    pub bucket: Bucket,
    pub best_secs: u32,
    pub best_mode: TravelMode,
}

pub struct Buckets {
    pub near: Vec<BucketEntry>,
    pub mid: Vec<BucketEntry>,
    pub far: Vec<BucketEntry>,
}

/// Assign all restaurants to buckets given their travel times.
/// Each restaurant appears in exactly one bucket (the nearest it qualifies for).
/// Restaurants that don't qualify for any bucket are silently excluded.
pub fn assign(restaurants: &[Restaurant], times: &HashMap<String, TravelTimes>) -> Buckets {
    let mut buckets = Buckets {
        near: Vec::new(),
        mid: Vec::new(),
        far: Vec::new(),
    };

    for restaurant in restaurants {
        let id = restaurant.id();
        let Some(travel_times) = times.get(&id) else {
            continue;
        };

        if let Some((bucket, best_secs, best_mode)) = classify(travel_times) {
            let vec = match bucket {
                Bucket::Near => &mut buckets.near,
                Bucket::Mid => &mut buckets.mid,
                Bucket::Far => &mut buckets.far,
            };
            vec.push(BucketEntry {
                restaurant: restaurant.clone(),
                bucket,
                best_secs,
                best_mode,
            });
        }
    }

    buckets
}

/// Determine which bucket a restaurant belongs to and what the best qualifying mode/time is.
/// Checks buckets in priority order (Near first), returns the first match.
///
/// Eligibility rules:
/// - Near (≤15 min): walk, bike, or transit
/// - Mid  (≤30 min): bike or transit only (not walk, not drive)
/// - Far  (≤60 min): bike, transit, or drive (not walk)
fn classify(times: &TravelTimes) -> Option<(Bucket, u32, TravelMode)> {
    // Near: walk, bike, or transit ≤ 15 min
    let near_candidates = [
        (times.walk_secs, TravelMode::Walk),
        (times.bike_secs, TravelMode::Bike),
        (times.transit_secs, TravelMode::Transit),
    ];
    if let Some((secs, mode)) = best_within(&near_candidates, NEAR_SECS) {
        return Some((Bucket::Near, secs, mode));
    }

    // Mid: bike or transit ≤ 30 min
    let mid_candidates = [
        (times.bike_secs, TravelMode::Bike),
        (times.transit_secs, TravelMode::Transit),
    ];
    if let Some((secs, mode)) = best_within(&mid_candidates, MID_SECS) {
        return Some((Bucket::Mid, secs, mode));
    }

    // Far: bike, transit, or drive ≤ 60 min
    let far_candidates = [
        (times.bike_secs, TravelMode::Bike),
        (times.transit_secs, TravelMode::Transit),
        (times.drive_secs, TravelMode::Drive),
    ];
    if let Some((secs, mode)) = best_within(&far_candidates, FAR_SECS) {
        return Some((Bucket::Far, secs, mode));
    }

    None
}

/// Return the (duration, mode) pair with the minimum duration among candidates within the limit.
fn best_within(candidates: &[(Option<u32>, TravelMode)], limit: u32) -> Option<(u32, TravelMode)> {
    candidates
        .iter()
        .filter_map(|(secs_opt, mode)| secs_opt.filter(|&s| s <= limit).map(|s| (s, *mode)))
        .min_by_key(|(s, _)| *s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times(walk: Option<u32>, bike: Option<u32>, transit: Option<u32>, drive: Option<u32>) -> TravelTimes {
        TravelTimes { walk_secs: walk, bike_secs: bike, transit_secs: transit, drive_secs: drive }
    }

    #[test]
    fn walk_under_15min_is_near() {
        let result = classify(&times(Some(800), None, None, None));
        assert!(matches!(result, Some((Bucket::Near, 800, TravelMode::Walk))));
    }

    #[test]
    fn exactly_900_secs_is_near() {
        let result = classify(&times(Some(900), None, None, None));
        assert!(matches!(result, Some((Bucket::Near, 900, _))));
    }

    #[test]
    fn walk_901_bike_within_mid_goes_to_mid() {
        // walk=901 (too far for near), bike=1500 qualifies for mid
        let result = classify(&times(Some(901), Some(1500), None, None));
        assert!(matches!(result, Some((Bucket::Mid, 1500, TravelMode::Bike))));
    }

    #[test]
    fn drive_only_within_60min_goes_to_far() {
        let result = classify(&times(None, None, None, Some(3000)));
        assert!(matches!(result, Some((Bucket::Far, 3000, TravelMode::Drive))));
    }

    #[test]
    fn drive_does_not_qualify_for_near_or_mid() {
        // drive=100 — drive is not eligible for Near or Mid
        let result = classify(&times(None, None, None, Some(100)));
        assert!(matches!(result, Some((Bucket::Far, 100, TravelMode::Drive))));
    }

    #[test]
    fn walk_does_not_qualify_for_mid_or_far() {
        // walk=1500 — walk is only eligible for Near
        let result = classify(&times(Some(1500), None, None, None));
        assert!(result.is_none());
    }

    #[test]
    fn too_far_is_excluded() {
        let result = classify(&times(Some(9999), Some(9999), Some(9999), Some(9999)));
        assert!(result.is_none());
    }

    #[test]
    fn all_none_is_excluded() {
        let result = classify(&times(None, None, None, None));
        assert!(result.is_none());
    }

    #[test]
    fn best_mode_is_fastest_eligible() {
        // bike=800, transit=500 — both qualify for Near, transit is faster
        let result = classify(&times(None, Some(800), Some(500), None));
        assert!(matches!(result, Some((Bucket::Near, 500, TravelMode::Transit))));
    }
}
