use chrono::{DateTime, Datelike, Timelike, Utc};

use super::models::WeeklyHours;

const MINS_PER_DAY: u32 = 24 * 60;

/// Determine whether a restaurant is open at a given UTC time,
/// using the restaurant's UTC offset to compute local time.
///
/// Fails open: if `hours` has no periods, returns `true`.
pub fn is_open_at(hours: &WeeklyHours, utc_offset_minutes: i32, now_utc: DateTime<Utc>) -> bool {
    if hours.periods.is_empty() {
        return true;
    }

    // Shift to restaurant local time
    let local = now_utc + chrono::Duration::minutes(utc_offset_minutes as i64);

    // chrono's weekday: Mon=0..Sun=6; Places API uses Sun=0..Sat=6
    let chrono_weekday = local.weekday().num_days_from_sunday(); // Sun=0, Sat=6
    let now_mins = chrono_weekday * MINS_PER_DAY + local.hour() * 60 + local.minute();

    for period in &hours.periods {
        // 24-hour restaurant: no close
        if period.close.is_none() {
            return true;
        }

        let close = period.close.as_ref().unwrap();

        let open_mins = period.open.day as u32 * MINS_PER_DAY
            + period.open.hour as u32 * 60
            + period.open.minute as u32;
        let close_mins =
            close.day as u32 * MINS_PER_DAY + close.hour as u32 * 60 + close.minute as u32;

        let open = if close_mins > open_mins {
            // Normal (same day or spans midnight within the week)
            now_mins >= open_mins && now_mins < close_mins
        } else {
            // Wraps around the week boundary (Sat night → Sun morning)
            now_mins >= open_mins || now_mins < close_mins
        };

        if open {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::places::models::{DayTime, HoursPeriod, WeeklyHours};
    use chrono::TimeZone;

    fn period(
        open_day: u8,
        open_h: u8,
        open_m: u8,
        close_day: u8,
        close_h: u8,
        close_m: u8,
    ) -> HoursPeriod {
        HoursPeriod {
            open: DayTime {
                day: open_day,
                hour: open_h,
                minute: open_m,
            },
            close: Some(DayTime {
                day: close_day,
                hour: close_h,
                minute: close_m,
            }),
        }
    }

    fn always_open() -> HoursPeriod {
        HoursPeriod {
            open: DayTime {
                day: 0,
                hour: 0,
                minute: 0,
            },
            close: None,
        }
    }

    /// UTC time that corresponds to a given local day/hour/min given offset.
    fn utc_for_local(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        offset_mins: i32,
    ) -> DateTime<Utc> {
        // local = utc + offset  =>  utc = local - offset
        let local_naive = chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            chrono::NaiveTime::from_hms_opt(hour, min, 0).unwrap(),
        );
        let utc_naive = local_naive - chrono::Duration::minutes(offset_mins as i64);
        Utc.from_utc_datetime(&utc_naive)
    }

    // 2026-04-06 is a Monday. Sun=0, Mon=1, ..., Sat=6

    #[test]
    fn open_during_weekday_lunch() {
        // Mon 11:00–22:00 UTC+0
        let hours = WeeklyHours {
            periods: vec![period(1, 11, 0, 1, 22, 0)],
        };
        // 2026-04-06 Monday 13:00 UTC+0
        let now = utc_for_local(2026, 4, 6, 13, 0, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn closed_at_3am() {
        // Mon 11:00–22:00 UTC+0
        let hours = WeeklyHours {
            periods: vec![period(1, 11, 0, 1, 22, 0)],
        };
        // 2026-04-06 Monday 03:00 UTC+0
        let now = utc_for_local(2026, 4, 6, 3, 0, 0);
        assert!(!is_open_at(&hours, 0, now));
    }

    #[test]
    fn twenty_four_hour_restaurant() {
        let hours = WeeklyHours {
            periods: vec![always_open()],
        };
        let now = utc_for_local(2026, 4, 6, 3, 30, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn empty_periods_fails_open() {
        let hours = WeeklyHours { periods: vec![] };
        let now = utc_for_local(2026, 4, 6, 3, 30, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn midnight_rollover_open() {
        // Fri(5) 22:00 – Sat(6) 02:00, UTC+0
        let hours = WeeklyHours {
            periods: vec![period(5, 22, 0, 6, 2, 0)],
        };
        // 2026-04-10 Fri 23:30 UTC+0
        let now = utc_for_local(2026, 4, 10, 23, 30, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn midnight_rollover_closed_before() {
        let hours = WeeklyHours {
            periods: vec![period(5, 22, 0, 6, 2, 0)],
        };
        // Fri 21:59
        let now = utc_for_local(2026, 4, 10, 21, 59, 0);
        assert!(!is_open_at(&hours, 0, now));
    }

    #[test]
    fn midnight_rollover_closed_after() {
        let hours = WeeklyHours {
            periods: vec![period(5, 22, 0, 6, 2, 0)],
        };
        // Sat 02:01
        let now = utc_for_local(2026, 4, 11, 2, 1, 0);
        assert!(!is_open_at(&hours, 0, now));
    }

    #[test]
    fn week_boundary_sat_to_sun_open() {
        // Sat(6) 22:00 – Sun(0) 02:00
        let hours = WeeklyHours {
            periods: vec![period(6, 22, 0, 0, 2, 0)],
        };
        // 2026-04-11 Sat 23:00
        let now = utc_for_local(2026, 4, 11, 23, 0, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn week_boundary_sat_to_sun_open_after_midnight() {
        // Sat(6) 22:00 – Sun(0) 02:00
        let hours = WeeklyHours {
            periods: vec![period(6, 22, 0, 0, 2, 0)],
        };
        // 2026-04-12 Sun 01:00
        let now = utc_for_local(2026, 4, 12, 1, 0, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn week_boundary_sat_to_sun_closed_midday_sunday() {
        // Sat(6) 22:00 – Sun(0) 02:00
        let hours = WeeklyHours {
            periods: vec![period(6, 22, 0, 0, 2, 0)],
        };
        // Sun 12:00 — closed
        let now = utc_for_local(2026, 4, 12, 12, 0, 0);
        assert!(!is_open_at(&hours, 0, now));
    }

    #[test]
    fn utc_offset_applied() {
        // Mon 11:00–22:00 local (UTC+300 minutes = UTC+5)
        let hours = WeeklyHours {
            periods: vec![period(1, 11, 0, 1, 22, 0)],
        };
        // Local 13:00 Mon (UTC+5) = 08:00 UTC → open
        let now = utc_for_local(2026, 4, 6, 13, 0, 300);
        assert!(is_open_at(&hours, 300, now));

        // Local 09:00 Mon (UTC+5) = 04:00 UTC → closed (before opening)
        let now_closed = utc_for_local(2026, 4, 6, 9, 0, 300);
        assert!(!is_open_at(&hours, 300, now_closed));
    }

    #[test]
    fn boundary_at_open_time_inclusive() {
        let hours = WeeklyHours {
            periods: vec![period(1, 11, 0, 1, 22, 0)],
        };
        let now = utc_for_local(2026, 4, 6, 11, 0, 0);
        assert!(is_open_at(&hours, 0, now));
    }

    #[test]
    fn boundary_at_close_time_exclusive() {
        let hours = WeeklyHours {
            periods: vec![period(1, 11, 0, 1, 22, 0)],
        };
        let now = utc_for_local(2026, 4, 6, 22, 0, 0);
        assert!(!is_open_at(&hours, 0, now));
    }
}
