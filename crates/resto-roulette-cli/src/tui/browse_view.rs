use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::app::{Action, App, Mode};
use super::widgets::{entry_detail_line, entry_name_line};
use resto_roulette_core::bucket::Bucket;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let (bucket_idx, cursor) = match &app.mode {
        Mode::Browse(s) => (s.bucket_idx, s.cursor),
        _ => return,
    };

    let bucket = match bucket_idx {
        0 => Bucket::Near,
        1 => Bucket::Mid,
        _ => Bucket::Far,
    };

    // Build list items from bucket entries (separate borrow from app.mode)
    let entries = match bucket_idx {
        0 => app.buckets.near.as_slice(),
        1 => app.buckets.mid.as_slice(),
        _ => app.buckets.far.as_slice(),
    };

    let item_count = entries.len();
    let title = format!(
        " {} {} {} ({} candidate{}) ",
        bucket.emoji(),
        bucket.mode_description(),
        bucket.label(),
        item_count,
        if item_count == 1 { "" } else { "s" }
    );

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_sel = i == cursor;
            let marker = if is_sel { "  ► " } else { "    " };
            let style = if is_sel {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(vec![
                entry_name_line(entry, marker, style),
                entry_detail_line(
                    entry,
                    Style::default().fg(if is_sel {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ),
                Line::raw(""),
            ])
        })
        .collect();

    let outer_block = Block::default().title(title).borders(Borders::ALL);
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    if items.is_empty() {
        let empty = Paragraph::new(Line::styled(
            "    (no restaurants in this range)",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(empty, chunks[0]);
    } else {
        let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));

        if let Mode::Browse(ref mut state) = app.mode {
            f.render_stateful_widget(list, chunks[0], &mut state.list_state);
        }
    }

    let footer = Line::styled(
        "  ↑↓/jk Scroll   Enter Pick   Esc Back",
        Style::default().fg(Color::DarkGray),
    );
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

pub fn handle_key(app: &mut App, key: KeyCode) -> Action {
    let (bucket_idx, cursor) = match &app.mode {
        Mode::Browse(s) => (s.bucket_idx, s.cursor),
        _ => return Action::Nothing,
    };

    let len = match bucket_idx {
        0 => app.buckets.near.len(),
        1 => app.buckets.mid.len(),
        _ => app.buckets.far.len(),
    };

    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Mode::Browse(ref mut state) = app.mode {
                state.move_up(len);
            }
            Action::Nothing
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Mode::Browse(ref mut state) = app.mode {
                state.move_down(len);
            }
            Action::Nothing
        }
        KeyCode::Enter => {
            let entry = match bucket_idx {
                0 => app.buckets.near.get(cursor),
                1 => app.buckets.mid.get(cursor),
                _ => app.buckets.far.get(cursor),
            }
            .cloned();
            if let Some(e) = entry {
                app.slots[bucket_idx] = Some(e);
                // Sync app.selected to the bucket that was just browsed
                app.selected = bucket_idx;
            }
            Action::SwitchToSlots
        }
        KeyCode::Esc => Action::SwitchToSlots,
        _ => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use resto_roulette_core::bucket::{Bucket, BucketEntry, Buckets};
    use resto_roulette_core::picker::Selection;
    use resto_roulette_core::routing::models::TravelMode;
    use resto_roulette_core::Restaurant;

    fn make_entry(name: &str) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: format!("{} St", name),
                location: None,
            },
            bucket: Bucket::Near,
            best_secs: 720,
            best_mode: TravelMode::Transit,
            cuisines: vec![],
        }
    }

    #[test]
    fn browse_draw_does_not_panic() {
        let buckets: &'static Buckets = Box::leak(Box::new(Buckets {
            near: vec![make_entry("Hà"), make_entry("Nouveau Palais")],
            mid: vec![],
            far: vec![],
        }));
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: None,
            far: None,
        };
        let mut app = App::new(buckets, initial);
        app.mode = Mode::Browse(super::super::app::BrowseState::new(0, 0));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
    }

    #[test]
    fn browse_draw_empty_bucket_does_not_panic() {
        let buckets: &'static Buckets = Box::leak(Box::new(Buckets {
            near: vec![],
            mid: vec![],
            far: vec![],
        }));
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let mut app = App::new(buckets, initial);
        app.mode = Mode::Browse(super::super::app::BrowseState::new(0, 0));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
    }

    #[test]
    fn browse_handle_key_enter_assigns_slot() {
        let buckets: &'static Buckets = Box::leak(Box::new(Buckets {
            near: vec![make_entry("Hà"), make_entry("Nouveau Palais")],
            mid: vec![],
            far: vec![],
        }));
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let mut app = App::new(buckets, initial);
        app.mode = Mode::Browse(super::super::app::BrowseState::new(0, 1)); // cursor at index 1

        let action = handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(action, Action::SwitchToSlots));
        assert_eq!(
            app.slots[0].as_ref().unwrap().restaurant.name,
            "Nouveau Palais"
        );
    }

    #[test]
    fn browse_handle_key_esc_returns_to_slots() {
        let buckets: &'static Buckets = Box::leak(Box::new(Buckets {
            near: vec![make_entry("Hà")],
            mid: vec![],
            far: vec![],
        }));
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let mut app = App::new(buckets, initial);
        app.mode = Mode::Browse(super::super::app::BrowseState::new(0, 0));
        let action = handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(action, Action::SwitchToSlots));
        // Slot should not have been assigned
        assert!(app.slots[0].is_none());
    }
}
