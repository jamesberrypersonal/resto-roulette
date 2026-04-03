use rand::Rng;

use crate::bucket::{BucketEntry, Buckets};

pub struct Selection {
    pub near: Option<BucketEntry>,
    pub mid: Option<BucketEntry>,
    pub far: Option<BucketEntry>,
}

/// Pick one entry from each bucket using the provided RNG.
/// Returns None for any bucket that is empty (non-fatal).
///
/// Note: deduplication across buckets is guaranteed by the bucketing logic —
/// each restaurant is assigned to exactly one bucket, so picks are always unique.
pub fn pick<R: Rng>(buckets: &Buckets, rng: &mut R) -> Selection {
    Selection {
        near: pick_one(&buckets.near, rng),
        mid: pick_one(&buckets.mid, rng),
        far: pick_one(&buckets.far, rng),
    }
}

/// Convenience wrapper using thread_rng for production use.
pub fn pick_random(buckets: &Buckets) -> Selection {
    pick(buckets, &mut rand::thread_rng())
}

fn pick_one<R: Rng>(entries: &[BucketEntry], rng: &mut R) -> Option<BucketEntry> {
    if entries.is_empty() {
        return None;
    }
    let idx = rng.gen_range(0..entries.len());
    Some(entries[idx].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::{Bucket, BucketEntry};
    use crate::routing::models::TravelMode;
    use crate::Restaurant;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

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
    fn empty_buckets_return_none() {
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
    fn single_entry_always_returned() {
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
}
