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
                let parens = format_entry_parens(entry);
                let line = match parens {
                    Some(p) => format!(
                        "   → {} ({})   {} by {}",
                        entry.restaurant.name,
                        p,
                        format_duration(entry.best_secs),
                        entry.best_mode.display_name(),
                    ),
                    None => format!(
                        "   → {}   {} by {}",
                        entry.restaurant.name,
                        format_duration(entry.best_secs),
                        entry.best_mode.display_name(),
                    ),
                };
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
        cuisines: &'a [String],
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
            cuisines: &entry.cuisines,
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

/// Build the parenthetical label for a bucket entry: "Cuisine · Address", "Cuisine",
/// "Address", or None when there's nothing useful to show.
pub(crate) fn format_entry_parens(entry: &BucketEntry) -> Option<String> {
    let has_real_address = entry.restaurant.address != entry.restaurant.name;
    let cuisine = entry.cuisines.first().map(|c| capitalize(c));
    match (cuisine, has_real_address) {
        (Some(c), true) => Some(format!("{} · {}", c, entry.restaurant.address)),
        (Some(c), false) => Some(c),
        (None, true) => Some(entry.restaurant.address.clone()),
        (None, false) => None,
    }
}

pub(crate) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub(crate) fn format_duration(secs: u32) -> String {
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
    use crate::bucket::Bucket;
    use crate::routing::models::TravelMode;
    use crate::Restaurant;

    fn make_entry(name: &str, address: &str, cuisines: Vec<String>) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: address.into(),
                location: None,
            },
            bucket: Bucket::Near,
            best_secs: 720,
            best_mode: TravelMode::Transit,
            cuisines,
        }
    }

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

    #[test]
    fn capitalize_lowercase_word() {
        assert_eq!(capitalize("vietnamese"), "Vietnamese");
    }

    #[test]
    fn capitalize_multi_word() {
        assert_eq!(capitalize("fast food"), "Fast food");
    }

    #[test]
    fn capitalize_empty_string() {
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn json_entry_includes_cuisines() {
        let entry = make_entry("Hà", "243 Rue De Bleury", vec!["vietnamese".into()]);
        assert_eq!(entry.cuisines, vec!["vietnamese"]);
    }

    #[test]
    fn json_entry_empty_cuisines() {
        let entry = make_entry("Resto", "123 Main St", vec![]);
        assert!(entry.cuisines.is_empty());
    }
}
