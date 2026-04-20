use ratatui::{style::Style, text::Line};

use crate::bucket::BucketEntry;
use crate::display::{format_duration, format_entry_parens};

/// Build the name line for a bucket entry: `"{marker}{name} ({parens})"`.
/// `marker` is typically `"  ► "` for selected or `"    "` for unselected.
pub fn entry_name_line(entry: &BucketEntry, marker: &str, style: Style) -> Line<'static> {
    let parens = format_entry_parens(entry);
    let text = match parens {
        Some(p) => format!("{}{} ({})", marker, entry.restaurant.name, p),
        None => format!("{}{}", marker, entry.restaurant.name),
    };
    Line::styled(text, style)
}

/// Build the travel detail line for a bucket entry: `"    {duration} by {mode}"`.
pub fn entry_detail_line(entry: &BucketEntry, style: Style) -> Line<'static> {
    Line::styled(
        format!(
            "    {} by {}",
            format_duration(entry.best_secs),
            entry.best_mode.display_name()
        ),
        style,
    )
}
