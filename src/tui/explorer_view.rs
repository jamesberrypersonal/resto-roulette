use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use super::app::{Action, App, Mode};
use super::widgets::entry_name_line;
use crate::bucket::{Bucket, BucketEntry};
use crate::display::{capitalize, format_duration};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let (active_bucket, search_query, search_active, sort_order) = match &app.mode {
        Mode::Explorer(s) => (
            s.active_bucket,
            s.search_query.clone(),
            s.search_active,
            s.sort_order,
        ),
        _ => return,
    };

    let outer_block = Block::default()
        .title(" resto-roulette — Explorer ")
        .borders(Borders::ALL);
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Vertical layout: tabs | search bar | main content | footer
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // bucket tabs
            Constraint::Length(1), // search / sort bar
            Constraint::Min(0),    // main content
            Constraint::Length(2), // footer
        ])
        .split(inner);

    // --- Bucket tabs ---
    let tab_titles = [Bucket::Near, Bucket::Mid, Bucket::Far]
        .iter()
        .map(|b| format!(" {} {} {} ", b.emoji(), b.mode_description(), b.label()))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(tab_titles)
        .select(active_bucket)
        .style(Style::default())
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw("│"));
    f.render_widget(tabs, sections[0]);

    // --- Search / sort bar ---
    let cursor_char = if search_active { "█" } else { "" };
    let search_text = format!("  / Search: {}{}", search_query, cursor_char);
    let sort_label = sort_order.label();
    // Pad to right-align the sort indicator
    let available = sections[1].width as usize;
    let sort_text = format!("Sort: {} ▲  ", sort_label);
    let pad = available.saturating_sub(search_text.len() + sort_text.len());
    let bar = format!("{}{:>pad$}{}", search_text, "", sort_text, pad = pad);
    let search_style = if search_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(Paragraph::new(Line::styled(bar, search_style)), sections[1]);

    // --- Main content: candidate list | detail panel ---
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Min(0)])
        .split(sections[2]);

    let entries = match active_bucket {
        0 => app.buckets.near.as_slice(),
        1 => app.buckets.mid.as_slice(),
        _ => app.buckets.far.as_slice(),
    };

    // Build filtered entry list and current highlighted entry before we mutably borrow mode
    let (filtered_indices, cursor) = match &app.mode {
        Mode::Explorer(s) => (s.filtered_indices.clone(), s.cursor),
        _ => return,
    };

    let list_items: Vec<ListItem> = filtered_indices
        .iter()
        .enumerate()
        .map(|(i, &src_idx)| {
            let entry = &entries[src_idx];
            let is_sel = i == cursor;
            let marker = if is_sel { "  ► " } else { "    " };
            let style = if is_sel {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            ListItem::new(entry_name_line(entry, marker, style))
        })
        .collect();

    // Candidate list block
    let list_count = filtered_indices.len();
    let list_title = if !search_query.is_empty() {
        format!(" Candidates ({}) ", list_count)
    } else {
        format!(" Candidates ({}) ", entries.len())
    };
    let list_block = Block::default().title(list_title).borders(Borders::ALL);
    let list_inner = list_block.inner(content_chunks[0]);
    f.render_widget(list_block, content_chunks[0]);

    if list_items.is_empty() {
        let msg = if search_query.is_empty() {
            "    (no restaurants in this range)"
        } else {
            "    (no matches)"
        };
        f.render_widget(
            Paragraph::new(Line::styled(
                msg.to_string(),
                Style::default().fg(Color::DarkGray),
            )),
            list_inner,
        );
    } else {
        let list =
            List::new(list_items).highlight_style(Style::default().add_modifier(Modifier::BOLD));
        if let Mode::Explorer(ref mut state) = app.mode {
            f.render_stateful_widget(list, list_inner, &mut state.list_state);
        }
    }

    // Detail panel
    let detail_block = Block::default().title(" Details ").borders(Borders::ALL);
    let detail_inner = detail_block.inner(content_chunks[1]);
    f.render_widget(detail_block, content_chunks[1]);

    let detail_lines = build_detail_lines(entries, &filtered_indices, cursor);
    f.render_widget(Paragraph::new(detail_lines), detail_inner);

    // --- Footer ---
    let footer_text = if search_active {
        "  Type to search   Backspace Delete   ↑↓ Scroll   Esc Clear   Enter Pick"
    } else {
        "  ←→/hl Bucket   ↑↓/jk Scroll   / Search   s Sort   Enter Pick   Esc Back"
    };
    let footer = Line::styled(footer_text, Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(footer), sections[3]);
}

fn build_detail_lines(
    entries: &[BucketEntry],
    filtered_indices: &[usize],
    cursor: usize,
) -> Vec<Line<'static>> {
    let entry = match filtered_indices.get(cursor).and_then(|&i| entries.get(i)) {
        Some(e) => e,
        None => {
            return vec![Line::styled(
                "  (no entry selected)".to_string(),
                Style::default().fg(Color::DarkGray),
            )]
        }
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default();

    let cuisine_display = if entry.cuisines.is_empty() {
        "—".to_string()
    } else {
        entry
            .cuisines
            .iter()
            .map(|c| capitalize(c))
            .collect::<Vec<_>>()
            .join(", ")
    };

    vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Name:    ".to_string(), label_style),
            Span::styled(entry.restaurant.name.clone(), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Address: ".to_string(), label_style),
            Span::styled(entry.restaurant.address.clone(), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Cuisine: ".to_string(), label_style),
            Span::styled(cuisine_display, value_style),
        ]),
        Line::from(vec![
            Span::styled("  Travel:  ".to_string(), label_style),
            Span::styled(
                format!(
                    "{} by {}",
                    format_duration(entry.best_secs),
                    entry.best_mode.display_name()
                ),
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Bucket:  ".to_string(), label_style),
            Span::styled(
                format!(
                    "{} ({})",
                    entry.bucket.mode_description(),
                    entry.bucket.label()
                ),
                value_style,
            ),
        ]),
    ]
}

pub fn handle_key(app: &mut App, key: KeyCode) -> Action {
    let (active_bucket, search_active) = match &app.mode {
        Mode::Explorer(s) => (s.active_bucket, s.search_active),
        _ => return Action::Nothing,
    };

    // When search input is active, most keys append to the query
    if search_active {
        match key {
            KeyCode::Esc => {
                // Clear search query and exit search mode
                let entries = match active_bucket {
                    0 => app.buckets.near.as_slice(),
                    1 => app.buckets.mid.as_slice(),
                    _ => app.buckets.far.as_slice(),
                };
                // We need entries before mutably borrowing app.mode
                let entries_owned: Vec<_> = entries.to_vec();
                if let Mode::Explorer(ref mut state) = app.mode {
                    state.search_query.clear();
                    state.search_active = false;
                    state.recompute_filtered(&entries_owned);
                }
                return Action::Nothing;
            }
            KeyCode::Backspace => {
                let entries = match active_bucket {
                    0 => app.buckets.near.as_slice(),
                    1 => app.buckets.mid.as_slice(),
                    _ => app.buckets.far.as_slice(),
                };
                let entries_owned: Vec<_> = entries.to_vec();
                if let Mode::Explorer(ref mut state) = app.mode {
                    state.search_query.pop();
                    state.recompute_filtered(&entries_owned);
                }
                return Action::Nothing;
            }
            KeyCode::Enter => {
                // Fall through to pick logic below (search_active -> enter picks)
            }
            KeyCode::Up => {
                if let Mode::Explorer(ref mut state) = app.mode {
                    state.move_up();
                }
                return Action::Nothing;
            }
            KeyCode::Down => {
                if let Mode::Explorer(ref mut state) = app.mode {
                    state.move_down();
                }
                return Action::Nothing;
            }
            KeyCode::Char(c) => {
                // All printable chars (including j/k) go into the search query
                let entries = match active_bucket {
                    0 => app.buckets.near.as_slice(),
                    1 => app.buckets.mid.as_slice(),
                    _ => app.buckets.far.as_slice(),
                };
                let entries_owned: Vec<_> = entries.to_vec();
                if let Mode::Explorer(ref mut state) = app.mode {
                    state.search_query.push(c);
                    state.recompute_filtered(&entries_owned);
                }
                return Action::Nothing;
            }
            _ => return Action::Nothing,
        }
    }

    // Normal (non-search) key handling
    match key {
        KeyCode::Esc => Action::SwitchToSlots,
        KeyCode::Enter => {
            // Pick the currently highlighted entry for its bucket
            let src_idx = match &app.mode {
                Mode::Explorer(s) => s.current_entry_idx(),
                _ => None,
            };
            if let Some(i) = src_idx {
                let entry = match active_bucket {
                    0 => app.buckets.near.get(i),
                    1 => app.buckets.mid.get(i),
                    _ => app.buckets.far.get(i),
                }
                .cloned();
                if let Some(e) = entry {
                    app.slots[active_bucket] = Some(e);
                    app.selected = active_bucket;
                }
            }
            Action::SwitchToSlots
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Mode::Explorer(ref mut state) = app.mode {
                state.move_up();
            }
            Action::Nothing
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Mode::Explorer(ref mut state) = app.mode {
                state.move_down();
            }
            Action::Nothing
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let new_bucket = active_bucket.saturating_sub(1);
            if new_bucket != active_bucket {
                switch_explorer_bucket(app, new_bucket);
            }
            Action::Nothing
        }
        KeyCode::Right | KeyCode::Char('l') => {
            let new_bucket = (active_bucket + 1).min(2);
            if new_bucket != active_bucket {
                switch_explorer_bucket(app, new_bucket);
            }
            Action::Nothing
        }
        KeyCode::Char('/') => {
            if let Mode::Explorer(ref mut state) = app.mode {
                state.search_active = true;
            }
            Action::Nothing
        }
        KeyCode::Char('s') => {
            let entries = match active_bucket {
                0 => app.buckets.near.as_slice(),
                1 => app.buckets.mid.as_slice(),
                _ => app.buckets.far.as_slice(),
            };
            let entries_owned: Vec<_> = entries.to_vec();
            if let Mode::Explorer(ref mut state) = app.mode {
                state.sort_order = state.sort_order.next();
                state.recompute_filtered(&entries_owned);
            }
            Action::Nothing
        }
        _ => Action::Nothing,
    }
}

fn switch_explorer_bucket(app: &mut App, new_bucket: usize) {
    let entries = match new_bucket {
        0 => app.buckets.near.as_slice(),
        1 => app.buckets.mid.as_slice(),
        _ => app.buckets.far.as_slice(),
    };
    let entries_owned: Vec<_> = entries.to_vec();
    if let Mode::Explorer(ref mut state) = app.mode {
        state.active_bucket = new_bucket;
        state.cursor = 0;
        state.search_query.clear();
        state.search_active = false;
        state.recompute_filtered(&entries_owned);
    }
}

#[cfg(test)]
mod tests {
    use super::super::app::SortOrder;
    use super::*;
    use crate::bucket::{Bucket, BucketEntry, Buckets};
    use crate::picker::Selection;
    use crate::routing::models::TravelMode;
    use crate::Restaurant;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn make_entry(name: &str, cuisine: Option<&str>) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: format!("{} St", name),
                location: None,
            },
            bucket: Bucket::Near,
            best_secs: 720,
            best_mode: TravelMode::Transit,
            cuisines: cuisine.map(|c| vec![c.to_string()]).unwrap_or_default(),
        }
    }

    fn make_app_in_explorer() -> App<'static> {
        let buckets: &'static Buckets = Box::leak(Box::new(Buckets {
            near: vec![
                make_entry("Hà", Some("vietnamese")),
                make_entry("Nouveau Palais", Some("diner")),
                make_entry("Joe Beef", None),
            ],
            mid: vec![make_entry("Le Diplomate", Some("french"))],
            far: vec![],
        }));
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: None,
            far: None,
        };
        App::new_in_explorer(buckets, initial)
    }

    #[test]
    fn explorer_draw_does_not_panic() {
        let mut app = make_app_in_explorer();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
    }

    #[test]
    fn explorer_draw_empty_bucket_does_not_panic() {
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
        let mut app = App::new_in_explorer(buckets, initial);
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &mut app, f.area())).unwrap();
    }

    #[test]
    fn explorer_handle_key_navigate_down() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Char('j'));
        let cursor = match &app.mode {
            Mode::Explorer(s) => s.cursor,
            _ => panic!("not in explorer"),
        };
        assert_eq!(cursor, 1);
    }

    #[test]
    fn explorer_handle_key_switch_bucket_right() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Right);
        let active = match &app.mode {
            Mode::Explorer(s) => s.active_bucket,
            _ => panic!("not in explorer"),
        };
        assert_eq!(active, 1);
    }

    #[test]
    fn explorer_handle_key_switch_bucket_left_clamps() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Left);
        let active = match &app.mode {
            Mode::Explorer(s) => s.active_bucket,
            _ => panic!("not in explorer"),
        };
        assert_eq!(active, 0); // already at 0, should not go negative
    }

    #[test]
    fn explorer_handle_key_search_filters() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Char('/'));
        handle_key(&mut app, KeyCode::Char('j'));
        handle_key(&mut app, KeyCode::Char('o'));
        handle_key(&mut app, KeyCode::Char('e'));
        let (query, count) = match &app.mode {
            Mode::Explorer(s) => (s.search_query.clone(), s.filtered_indices.len()),
            _ => panic!("not in explorer"),
        };
        assert_eq!(query, "joe");
        assert_eq!(count, 1); // Only "Joe Beef" matches "joe"
    }

    #[test]
    fn explorer_handle_key_search_backspace() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Char('/'));
        handle_key(&mut app, KeyCode::Char('h'));
        handle_key(&mut app, KeyCode::Char('a'));
        handle_key(&mut app, KeyCode::Backspace);
        let (query, count) = match &app.mode {
            Mode::Explorer(s) => (s.search_query.clone(), s.filtered_indices.len()),
            _ => panic!("not in explorer"),
        };
        assert_eq!(query, "h");
        // "h" matches "Hà" only
        assert_eq!(count, 1);
    }

    #[test]
    fn explorer_handle_key_search_esc_clears() {
        let mut app = make_app_in_explorer();
        handle_key(&mut app, KeyCode::Char('/'));
        handle_key(&mut app, KeyCode::Char('h'));
        handle_key(&mut app, KeyCode::Esc); // clears search
        let (query, active, count) = match &app.mode {
            Mode::Explorer(s) => (
                s.search_query.clone(),
                s.search_active,
                s.filtered_indices.len(),
            ),
            _ => panic!("not in explorer"),
        };
        assert_eq!(query, "");
        assert!(!active);
        assert_eq!(count, 3); // all near entries restored
    }

    #[test]
    fn explorer_handle_key_esc_exits_to_slots() {
        let mut app = make_app_in_explorer();
        let action = handle_key(&mut app, KeyCode::Esc);
        assert!(matches!(action, Action::SwitchToSlots));
    }

    #[test]
    fn explorer_handle_key_enter_assigns_slot() {
        let mut app = make_app_in_explorer();
        // Cursor is at 0 (Hà) by default
        let action = handle_key(&mut app, KeyCode::Enter);
        assert!(matches!(action, Action::SwitchToSlots));
        assert_eq!(app.slots[0].as_ref().unwrap().restaurant.name, "Hà");
    }

    #[test]
    fn explorer_handle_key_sort_cycles() {
        let mut app = make_app_in_explorer();
        let initial_sort = match &app.mode {
            Mode::Explorer(s) => s.sort_order,
            _ => panic!(),
        };
        assert_eq!(initial_sort, SortOrder::Name);
        handle_key(&mut app, KeyCode::Char('s'));
        let new_sort = match &app.mode {
            Mode::Explorer(s) => s.sort_order,
            _ => panic!(),
        };
        assert_eq!(new_sort, SortOrder::TravelTime);
    }
}
