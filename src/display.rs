use colored::Colorize;
use serde::Serialize;

use crate::bucket::{Bucket, BucketEntry};
use crate::config::OutputFormat;
use crate::picker::Selection;

pub fn render(selection: &Selection, format: OutputFormat) {
    match format {
        OutputFormat::Pretty => render_pretty(selection),
        OutputFormat::Json => render_json(selection),
    }
}

fn render_pretty(selection: &Selection) {
    let slots = [
        (Bucket::Near, &selection.near),
        (Bucket::Mid, &selection.mid),
        (Bucket::Far, &selection.far),
    ];

    for (bucket, entry_opt) in &slots {
        let header = format!(
            "{} {} {}",
            bucket.emoji(),
            bucket.mode_description(),
            bucket.label()
        );
        println!("{}", header.bold());

        match entry_opt {
            Some(entry) => {
                let line = format!(
                    "   → {} ({})   {}",
                    entry.restaurant.name,
                    entry.restaurant.address,
                    format_duration(entry.best_secs),
                );
                println!("{}", line);
            }
            None => {
                println!("{}", "   (no restaurant found in this range)".dimmed());
            }
        }

        println!();
    }
}

fn render_json(selection: &Selection) {
    #[derive(Serialize)]
    struct JsonEntry<'a> {
        name: &'a str,
        address: &'a str,
        bucket: &'a str,
        best_mode: &'a str,
        best_secs: u32,
    }

    #[derive(Serialize)]
    struct JsonOutput<'a> {
        near: Option<JsonEntry<'a>>,
        mid: Option<JsonEntry<'a>>,
        far: Option<JsonEntry<'a>>,
    }

    fn to_json_entry(entry: &BucketEntry) -> JsonEntry<'_> {
        JsonEntry {
            name: &entry.restaurant.name,
            address: &entry.restaurant.address,
            bucket: entry.bucket.label(),
            best_mode: entry.best_mode.display_name(),
            best_secs: entry.best_secs,
        }
    }

    let output = JsonOutput {
        near: selection.near.as_ref().map(to_json_entry),
        mid: selection.mid.as_ref().map(to_json_entry),
        far: selection.far.as_ref().map(to_json_entry),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
}

fn format_duration(secs: u32) -> String {
    let mins = (secs + 30) / 60; // round to nearest minute
    if mins < 60 {
        format!("~{} min", mins)
    } else {
        let h = mins / 60;
        let m = mins % 60;
        if m == 0 {
            format!("~{} h", h)
        } else {
            format!("~{} h {} min", h, m)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(720), "~12 min");
    }

    #[test]
    fn format_duration_hours_and_minutes() {
        assert_eq!(format_duration(3900), "~1 h 5 min");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(3600), "~1 h");
    }
}
