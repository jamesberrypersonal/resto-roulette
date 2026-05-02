use std::collections::HashMap;
use std::path::PathBuf;

use chrono::Utc;
use futures::StreamExt;

use crate::bucket::{self, Buckets};
use crate::cache::Cache;
use crate::error::AppError;
use crate::parse;
use crate::places::{self, PlacesClient};
use crate::routing::RoutingClient;

pub struct PipelineInputs {
    pub list_path: PathBuf,
    pub home: String,
    pub api_key: String,
    pub dry_run: bool,
    pub enrich: EnrichOpts,
}

pub struct EnrichOpts {
    pub open_now: bool,
    /// Empty means no include-filter.
    pub cuisine_filter: Vec<String>,
    pub exclude_cuisines: Vec<String>,
}

impl EnrichOpts {
    pub fn server_v1() -> Self {
        EnrichOpts {
            open_now: false,
            cuisine_filter: vec![],
            exclude_cuisines: vec![],
        }
    }
}

pub async fn run(inputs: &PipelineInputs, cache: &Cache) -> Result<Buckets, AppError> {
    let restaurants = parse::parse_file(&inputs.list_path).map_err(|e| {
        tracing::error!("Failed to parse {:?}: {}", inputs.list_path, e);
        e
    })?;
    tracing::info!("Loaded {} restaurants", restaurants.len());

    if restaurants.is_empty() {
        tracing::warn!("No restaurants found in {:?}", inputs.list_path);
        return Ok(Buckets {
            near: vec![],
            mid: vec![],
            far: vec![],
        });
    }

    let home_id = crate::cache::sqlite::hash_home(&inputs.home);
    let client = RoutingClient::new(inputs.api_key.clone())?;

    let needs_enrichment = inputs.enrich.open_now
        || !inputs.enrich.cuisine_filter.is_empty()
        || !inputs.enrich.exclude_cuisines.is_empty();

    let place_details_map: HashMap<String, places::PlaceDetails> = if needs_enrichment {
        let places_client = PlacesClient::new(inputs.api_key.clone())?;

        let restaurant_data: Vec<_> = restaurants
            .iter()
            .map(|r| (r.id(), r.name.clone(), r.address.clone()))
            .collect();
        futures::stream::iter(restaurant_data)
            .map(|(rid, name, address)| {
                let dry_run = inputs.dry_run;
                let cache = &cache;
                let places_client = &places_client;
                async move {
                    let cached = cache.get_place(&rid, dry_run).unwrap_or(None);

                    let details = match cached {
                        Some((details, true)) => {
                            tracing::debug!("Place cache hit (fresh) for '{}'", name);
                            Some(details)
                        }
                        Some((details, false)) if dry_run => {
                            tracing::debug!("Dry run: using stale place cache for '{}'", name);
                            Some(details)
                        }
                        Some((stale, false)) => {
                            tracing::debug!(
                                "Place cache stale for '{}', refreshing via Place Details",
                                name
                            );
                            let refreshed = places_client
                                .place_details(&stale.place_id)
                                .await
                                .unwrap_or_else(|e| {
                                    tracing::warn!(
                                        "Failed to refresh place details for '{}': {}",
                                        name,
                                        e
                                    );
                                    None
                                });
                            let result = refreshed.or(Some(stale));
                            if let Some(ref d) = result {
                                if let Err(e) = cache.put_place(&rid, d) {
                                    tracing::warn!(
                                        "Failed to cache place details for '{}': {}",
                                        name,
                                        e
                                    );
                                }
                            }
                            result
                        }
                        None if dry_run => {
                            tracing::debug!(
                                "Dry run: no place cache for '{}', keeping (fail-open)",
                                name
                            );
                            None
                        }
                        None => {
                            tracing::debug!("Fetching place details for '{}'", name);
                            let fetched = places_client
                                .text_search(&name, &address)
                                .await
                                .unwrap_or_else(|e| {
                                    tracing::warn!(
                                        "Failed to fetch place details for '{}': {}",
                                        name,
                                        e
                                    );
                                    None
                                });
                            if let Some(ref d) = fetched {
                                if let Err(e) = cache.put_place(&rid, d) {
                                    tracing::warn!(
                                        "Failed to cache place details for '{}': {}",
                                        name,
                                        e
                                    );
                                }
                            }
                            fetched
                        }
                    };

                    (rid, details)
                }
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .filter_map(|(rid, opt)| opt.map(|d| (rid, d)))
            .collect()
    } else {
        restaurants
            .iter()
            .filter_map(|r| {
                let rid = r.id();
                cache
                    .get_place(&rid, true)
                    .unwrap_or(None)
                    .map(|(details, _)| (rid, details))
            })
            .collect()
    };

    let restaurants = if inputs.enrich.open_now {
        let now = Utc::now();
        let original_len = restaurants.len();
        let open: Vec<_> = restaurants
            .into_iter()
            .filter(|r| match place_details_map.get(&r.id()) {
                Some(d) => match (&d.hours, d.utc_offset_minutes) {
                    (Some(h), Some(offset)) => places::hours::is_open_at(h, offset, now),
                    _ => true,
                },
                None => true,
            })
            .collect();
        let skipped = original_len - open.len();
        if skipped > 0 {
            tracing::info!(
                "Skipped {} closed restaurant{}.",
                skipped,
                if skipped == 1 { "" } else { "s" }
            );
        }
        if open.is_empty() {
            tracing::warn!("No open restaurants found. Try without --open-now.");
            return Ok(Buckets {
                near: vec![],
                mid: vec![],
                far: vec![],
            });
        }
        open
    } else {
        restaurants
    };

    let cuisine_map: HashMap<String, Vec<String>> = place_details_map
        .iter()
        .map(|(rid, d)| (rid.clone(), places::extract_cuisines(&d.types)))
        .collect();

    let restaurants =
        if !inputs.enrich.cuisine_filter.is_empty() || !inputs.enrich.exclude_cuisines.is_empty() {
            let original_len = restaurants.len();
            let filtered: Vec<_> = restaurants
                .into_iter()
                .filter(|r| {
                    let cuisines = cuisine_map
                        .get(&r.id())
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    if cuisines.is_empty() {
                        return true;
                    }
                    if !inputs.enrich.cuisine_filter.is_empty() {
                        cuisines
                            .iter()
                            .any(|c| inputs.enrich.cuisine_filter.contains(c))
                    } else {
                        !cuisines
                            .iter()
                            .any(|c| inputs.enrich.exclude_cuisines.contains(c))
                    }
                })
                .collect();
            let skipped = original_len - filtered.len();
            if skipped > 0 {
                tracing::info!(
                    "Skipped {} restaurant{} by cuisine filter.",
                    skipped,
                    if skipped == 1 { "" } else { "s" }
                );
            }
            if filtered.is_empty() {
                tracing::warn!("No restaurants matched the cuisine filter.");
                return Ok(Buckets {
                    near: vec![],
                    mid: vec![],
                    far: vec![],
                });
            }
            filtered
        } else {
            restaurants
        };

    let restaurant_data2: Vec<_> = restaurants
        .iter()
        .map(|r| (r.id(), r.name.clone(), r.address.clone()))
        .collect();
    let all_times: HashMap<String, _> = futures::stream::iter(restaurant_data2)
        .map(|(rid, name, address)| {
            let dry_run = inputs.dry_run;
            let home = &inputs.home;
            let home_id = &home_id;
            let cache = &cache;
            let client = &client;
            async move {
                let cached = cache.get(&rid, home_id, dry_run).unwrap_or_default();

                let times = if cached.is_complete() {
                    tracing::debug!("Cache hit for '{}'", name);
                    cached
                } else if dry_run {
                    tracing::debug!("Dry run: using partial/empty cache for '{}'", name);
                    cached
                } else {
                    tracing::debug!("Fetching travel times for '{}'", name);
                    let fetched = client
                        .get_travel_times(home, &address)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to fetch times for '{}': {}", name, e);
                            Default::default()
                        });
                    if let Err(e) = cache.put(&rid, home_id, &fetched) {
                        tracing::warn!("Failed to cache times for '{}': {}", name, e);
                    }
                    fetched
                };

                (rid, times)
            }
        })
        .buffer_unordered(10)
        .collect()
        .await;

    let buckets = bucket::assign(&restaurants, &all_times, &cuisine_map);

    if buckets.near.is_empty() && buckets.mid.is_empty() && buckets.far.is_empty() {
        tracing::warn!(
            "No restaurants could be bucketed (try running without --dry-run to fetch travel times)."
        );
    }

    Ok(buckets)
}
