use std::collections::HashMap;

use resto_roulette_core::bucket::assign;
use resto_roulette_core::routing::models::TravelTimes;
use resto_roulette_core::Restaurant;

fn restaurant(name: &str) -> Restaurant {
    Restaurant {
        name: name.into(),
        address: "addr".into(),
        location: None,
    }
}

fn times(
    walk: Option<u32>,
    bike: Option<u32>,
    transit: Option<u32>,
    drive: Option<u32>,
) -> TravelTimes {
    TravelTimes {
        walk_secs: walk,
        bike_secs: bike,
        transit_secs: transit,
        drive_secs: drive,
    }
}

fn single_bucket(
    walk: Option<u32>,
    bike: Option<u32>,
    transit: Option<u32>,
    drive: Option<u32>,
) -> (usize, usize, usize) {
    let r = restaurant("R");
    let id = r.id();
    let mut map = HashMap::new();
    map.insert(id, times(walk, bike, transit, drive));
    let b = assign(&[r], &map, &HashMap::new());
    (b.near.len(), b.mid.len(), b.far.len())
}

#[test]
fn walk_under_15min_goes_to_near() {
    assert_eq!(single_bucket(Some(800), None, None, None), (1, 0, 0));
}

#[test]
fn exactly_900_secs_is_near() {
    assert_eq!(single_bucket(Some(900), None, None, None), (1, 0, 0));
}

#[test]
fn walk_901_bike_in_mid_range_goes_to_mid() {
    // walk=901 (too far for near), bike=1500 qualifies for mid
    assert_eq!(single_bucket(Some(901), Some(1500), None, None), (0, 1, 0));
}

#[test]
fn drive_only_within_60min_goes_to_far() {
    assert_eq!(single_bucket(None, None, None, Some(3000)), (0, 0, 1));
}

#[test]
fn too_far_is_excluded() {
    assert_eq!(
        single_bucket(Some(9999), Some(9999), Some(9999), Some(9999)),
        (0, 0, 0)
    );
}

#[test]
fn all_none_is_excluded() {
    assert_eq!(single_bucket(None, None, None, None), (0, 0, 0));
}

#[test]
fn walk_in_near_range_wins_over_mid_for_same_restaurant() {
    // walk=500, bike=1000 — both modes qualify for Near, so restaurant goes to Near not Mid
    assert_eq!(single_bucket(Some(500), Some(1000), None, None), (1, 0, 0));
}

#[test]
fn drive_under_30min_excluded_from_all_buckets() {
    // drive=100 — drive is not eligible for Near/Mid, and too fast to qualify for Far
    assert_eq!(single_bucket(None, None, None, Some(100)), (0, 0, 0));
}

#[test]
fn drive_over_30min_qualifies_for_far() {
    // drive=2100 (35 min) — above Mid floor, eligible for Far
    assert_eq!(single_bucket(None, None, None, Some(2100)), (0, 0, 1));
}

#[test]
fn transit_qualifies_for_near() {
    assert_eq!(single_bucket(None, None, Some(600), None), (1, 0, 0));
}

#[test]
fn multiple_restaurants_distributed_correctly() {
    let r_near = restaurant("Near");
    let r_mid = restaurant("Mid");
    let r_far = restaurant("Far");
    let r_excluded = restaurant("Excluded");

    let mut map = HashMap::new();
    map.insert(r_near.id(), times(Some(500), None, None, None));
    map.insert(r_mid.id(), times(None, Some(1500), None, None));
    map.insert(r_far.id(), times(None, None, None, Some(3000)));
    map.insert(
        r_excluded.id(),
        times(Some(9999), Some(9999), Some(9999), Some(9999)),
    );

    let buckets = assign(&[r_near, r_mid, r_far, r_excluded], &map, &HashMap::new());
    assert_eq!(buckets.near.len(), 1);
    assert_eq!(buckets.mid.len(), 1);
    assert_eq!(buckets.far.len(), 1);
}
