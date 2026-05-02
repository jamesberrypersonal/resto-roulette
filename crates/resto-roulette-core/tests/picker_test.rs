use rand::rngs::StdRng;
use rand::SeedableRng;

use resto_roulette_core::bucket::{Bucket, BucketEntry, Buckets};
use resto_roulette_core::picker::pick;
use resto_roulette_core::routing::models::TravelMode;
use resto_roulette_core::Restaurant;

fn entry(name: &str, bucket: Bucket) -> BucketEntry {
    BucketEntry {
        restaurant: Restaurant {
            name: name.into(),
            address: "addr".into(),
            location: None,
        },
        bucket,
        best_secs: 600,
        best_mode: TravelMode::Walk,
        cuisines: vec![],
    }
}

fn make_buckets() -> Buckets {
    Buckets {
        near: vec![entry("Near1", Bucket::Near), entry("Near2", Bucket::Near)],
        mid: vec![entry("Mid1", Bucket::Mid)],
        far: vec![],
    }
}

#[test]
fn picks_one_from_each_non_empty_bucket() {
    let mut rng = StdRng::seed_from_u64(42);
    let buckets = make_buckets();
    let selection = pick(&buckets, &mut rng);
    assert!(selection.near.is_some());
    assert!(selection.mid.is_some());
    assert!(selection.far.is_none());
}

#[test]
fn empty_far_bucket_returns_none() {
    let mut rng = StdRng::seed_from_u64(42);
    let selection = pick(&make_buckets(), &mut rng);
    assert!(selection.far.is_none());
}

#[test]
fn seeded_pick_is_deterministic() {
    let buckets = make_buckets();
    let s1 = pick(&buckets, &mut StdRng::seed_from_u64(42));
    let s2 = pick(&buckets, &mut StdRng::seed_from_u64(42));
    assert_eq!(
        s1.near.unwrap().restaurant.name,
        s2.near.unwrap().restaurant.name
    );
}

#[test]
fn different_seeds_can_produce_different_picks() {
    let buckets = make_buckets();
    // With two entries in near bucket and many seeds, we should see both picked at least once
    let names: std::collections::HashSet<String> = (0u64..50)
        .map(|seed| {
            pick(&buckets, &mut StdRng::seed_from_u64(seed))
                .near
                .unwrap()
                .restaurant
                .name
        })
        .collect();
    assert!(
        names.len() > 1,
        "expected both Near1 and Near2 to be picked across 50 seeds"
    );
}

#[test]
fn all_empty_buckets_all_none() {
    let buckets = Buckets {
        near: vec![],
        mid: vec![],
        far: vec![],
    };
    let mut rng = StdRng::seed_from_u64(0);
    let selection = pick(&buckets, &mut rng);
    assert!(selection.near.is_none());
    assert!(selection.mid.is_none());
    assert!(selection.far.is_none());
}

#[test]
fn single_entry_always_picked() {
    let buckets = Buckets {
        near: vec![entry("Only", Bucket::Near)],
        mid: vec![],
        far: vec![],
    };
    for seed in 0..20u64 {
        let mut rng = StdRng::seed_from_u64(seed);
        let selection = pick(&buckets, &mut rng);
        assert_eq!(selection.near.unwrap().restaurant.name, "Only");
    }
}
