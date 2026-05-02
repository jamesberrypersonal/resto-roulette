use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use hex;
use rusqlite::{params, Connection};
use serde_json;
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::places::models::PlaceDetails;
use crate::routing::models::{TravelMode, TravelTimes};

pub struct Cache {
    // Wrapped in Mutex so that &Cache: Send — required for the server's async pipeline futures.
    // rusqlite::Connection is Send but !Sync; Mutex<Connection> is both Send and Sync.
    conn: std::sync::Mutex<Connection>,
    ttl: Duration,
    places_ttl: Duration,
}

impl Cache {
    pub fn open(db_path: &Path, ttl_hours: u64, places_ttl_hours: u64) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS travel_times (
                restaurant_id TEXT NOT NULL,
                home_id       TEXT NOT NULL,
                mode          TEXT NOT NULL,
                duration_secs INTEGER NOT NULL,
                fetched_at    TEXT NOT NULL,
                PRIMARY KEY (restaurant_id, home_id, mode)
            );
            CREATE TABLE IF NOT EXISTS place_details (
                restaurant_id      TEXT PRIMARY KEY,
                place_id           TEXT NOT NULL,
                types_json         TEXT NOT NULL,
                hours_json         TEXT,
                utc_offset_minutes INTEGER,
                fetched_at         TEXT NOT NULL
            );",
        )?;

        Ok(Self {
            conn: std::sync::Mutex::new(conn),
            ttl: Duration::hours(ttl_hours as i64),
            places_ttl: Duration::hours(places_ttl_hours as i64),
        })
    }

    /// Look up all four travel modes for a (restaurant, home) pair.
    /// Returns None for modes not in cache or whose entries are expired (unless dry_run=true).
    pub fn get(&self, restaurant_id: &str, home_id: &str, dry_run: bool) -> Result<TravelTimes> {
        let cutoff: Option<DateTime<Utc>> = if dry_run {
            None // accept any cached entry regardless of age
        } else {
            Some(Utc::now() - self.ttl)
        };

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mode, duration_secs, fetched_at
             FROM travel_times
             WHERE restaurant_id = ?1 AND home_id = ?2",
        )?;

        let rows = stmt.query_map(params![restaurant_id, home_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut times = TravelTimes::default();

        for row in rows {
            let (mode_str, duration_secs, fetched_at_str) = row?;

            if let Some(cutoff) = cutoff {
                let fetched_at = fetched_at_str
                    .parse::<DateTime<Utc>>()
                    .unwrap_or(DateTime::<Utc>::MIN_UTC);
                if fetched_at < cutoff {
                    tracing::debug!("Cache entry for mode '{}' is expired", mode_str);
                    continue;
                }
            }

            match TravelMode::from_db_str(&mode_str) {
                Some(TravelMode::Walk) => times.walk_secs = Some(duration_secs),
                Some(TravelMode::Bike) => times.bike_secs = Some(duration_secs),
                Some(TravelMode::Transit) => times.transit_secs = Some(duration_secs),
                Some(TravelMode::Drive) => times.drive_secs = Some(duration_secs),
                None => tracing::warn!("Unknown travel mode in cache: '{}'", mode_str),
            }
        }

        Ok(times)
    }

    /// Write travel times for all four modes (upsert).
    pub fn put(&self, restaurant_id: &str, home_id: &str, times: &TravelTimes) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        let entries: &[(Option<u32>, TravelMode)] = &[
            (times.walk_secs, TravelMode::Walk),
            (times.bike_secs, TravelMode::Bike),
            (times.transit_secs, TravelMode::Transit),
            (times.drive_secs, TravelMode::Drive),
        ];

        let conn = self.conn.lock().unwrap();
        for (secs_opt, mode) in entries {
            if let Some(secs) = secs_opt {
                conn.execute(
                    "INSERT OR REPLACE INTO travel_times
                     (restaurant_id, home_id, mode, duration_secs, fetched_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![restaurant_id, home_id, mode.db_str(), secs, now],
                )?;
            }
        }

        Ok(())
    }

    /// Look up cached place details for a restaurant.
    ///
    /// Returns `Some((details, is_fresh))` on a hit:
    /// - `is_fresh = true`  → entry is within TTL, use as-is.
    /// - `is_fresh = false` → entry is stale; caller may refresh or use as fallback.
    ///
    /// Returns `None` on a cache miss.
    ///
    /// When `dry_run = true`, TTL is ignored and any cached entry is returned as fresh.
    pub fn get_place(
        &self,
        restaurant_id: &str,
        dry_run: bool,
    ) -> Result<Option<(PlaceDetails, bool)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT place_id, types_json, hours_json, utc_offset_minutes, fetched_at
             FROM place_details
             WHERE restaurant_id = ?1",
        )?;

        let row = stmt.query_row(params![restaurant_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i32>>(3)?,
                row.get::<_, String>(4)?,
            ))
        });

        match row {
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
            Ok((place_id, types_json, hours_json, utc_offset_minutes, fetched_at_str)) => {
                let types: Vec<String> = serde_json::from_str(&types_json)?;
                let hours = hours_json
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?;

                let details = PlaceDetails {
                    place_id,
                    types,
                    hours,
                    utc_offset_minutes,
                };

                let is_fresh = if dry_run {
                    true
                } else {
                    let fetched_at = fetched_at_str
                        .parse::<DateTime<Utc>>()
                        .unwrap_or(DateTime::<Utc>::MIN_UTC);
                    fetched_at >= Utc::now() - self.places_ttl
                };

                Ok(Some((details, is_fresh)))
            }
        }
    }

    /// Write place details for a restaurant (upsert).
    pub fn put_place(&self, restaurant_id: &str, details: &PlaceDetails) -> Result<()> {
        let types_json = serde_json::to_string(&details.types)?;
        let hours_json = details
            .hours
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let now = Utc::now().to_rfc3339();

        self.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO place_details
             (restaurant_id, place_id, types_json, hours_json, utc_offset_minutes, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                restaurant_id,
                details.place_id,
                types_json,
                hours_json,
                details.utc_offset_minutes,
                now
            ],
        )?;

        Ok(())
    }

    /// Delete expired place_details entries. Call at startup to keep the file small.
    pub fn evict_expired_places(&self) -> Result<usize> {
        let cutoff = (Utc::now() - self.places_ttl).to_rfc3339();
        let deleted = self.conn.lock().unwrap().execute(
            "DELETE FROM place_details WHERE fetched_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    /// Delete expired entries. Call at startup to keep the file small.
    pub fn evict_expired(&self) -> Result<usize> {
        let cutoff = (Utc::now() - self.ttl).to_rfc3339();
        let deleted = self.conn.lock().unwrap().execute(
            "DELETE FROM travel_times WHERE fetched_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }
}

/// Stable SHA-256 fingerprint of name+address used as primary key.
/// Null byte separator prevents collisions between ("ab","c") and ("a","bc").
pub fn hash_restaurant(name: &str, address: &str) -> String {
    let mut h = Sha256::new();
    h.update(name.as_bytes());
    h.update(b"\x00");
    h.update(address.as_bytes());
    hex::encode(h.finalize())
}

pub fn hash_home(home: &str) -> String {
    hex::encode(Sha256::digest(home.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Returns both the Cache and the TempDir; drop order matters —
    /// TempDir must outlive Cache so the DB file is valid for the whole test.
    fn temp_cache(ttl_hours: u64) -> (Cache, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let cache = Cache::open(&path, ttl_hours, 720).unwrap();
        (cache, dir)
    }

    #[test]
    fn round_trip_write_and_read() {
        let (cache, _dir) = temp_cache(168);
        let rid = "restaurant1";
        let hid = "home1";
        let times = TravelTimes {
            walk_secs: Some(300),
            bike_secs: Some(600),
            transit_secs: Some(450),
            drive_secs: Some(900),
        };

        cache.put(rid, hid, &times).unwrap();
        let result = cache.get(rid, hid, false).unwrap();

        assert_eq!(result.walk_secs, Some(300));
        assert_eq!(result.bike_secs, Some(600));
        assert_eq!(result.transit_secs, Some(450));
        assert_eq!(result.drive_secs, Some(900));
    }

    #[test]
    fn dry_run_returns_expired_entries() {
        let (cache, _dir) = temp_cache(0); // TTL=0 means everything expires immediately
        let rid = "r";
        let hid = "h";
        let times = TravelTimes {
            walk_secs: Some(500),
            ..Default::default()
        };
        cache.put(rid, hid, &times).unwrap();

        // With dry_run=false, expired entries are filtered out
        let result_normal = cache.get(rid, hid, false).unwrap();
        // May or may not be expired depending on timing — just check dry_run=true works

        // With dry_run=true, expired entries are still returned
        let result_dry = cache.get(rid, hid, true).unwrap();
        assert_eq!(result_dry.walk_secs, Some(500));
        let _ = result_normal; // timing-dependent, just ensure no panic
    }

    #[test]
    fn missing_entry_returns_none() {
        let (cache, _dir) = temp_cache(168);
        let result = cache.get("nonexistent", "home", false).unwrap();
        assert!(result.walk_secs.is_none());
        assert!(result.bike_secs.is_none());
    }

    #[test]
    fn hash_restaurant_separator_prevents_collision() {
        let h1 = hash_restaurant("ab", "c");
        let h2 = hash_restaurant("a", "bc");
        assert_ne!(h1, h2);
    }

    #[test]
    fn upsert_overwrites_previous_value() {
        let (cache, _dir) = temp_cache(168);
        let rid = "r";
        let hid = "h";
        cache
            .put(
                rid,
                hid,
                &TravelTimes {
                    walk_secs: Some(300),
                    ..Default::default()
                },
            )
            .unwrap();
        cache
            .put(
                rid,
                hid,
                &TravelTimes {
                    walk_secs: Some(999),
                    ..Default::default()
                },
            )
            .unwrap();
        let result = cache.get(rid, hid, false).unwrap();
        assert_eq!(result.walk_secs, Some(999));
    }
}
