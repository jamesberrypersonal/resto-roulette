use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::bucket::Bucket;

use super::app::{Action, App, BrowseState, ExplorerState};
use super::widgets::{entry_detail_line, entry_name_line};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let outer_block = Block::default()
        .title(" resto-roulette ")
        .borders(Borders::ALL);
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let buckets_info = [
        (Bucket::Near, &app.slots[0], 0usize),
        (Bucket::Mid, &app.slots[1], 1),
        (Bucket::Far, &app.slots[2], 2),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (bucket, entry_opt, idx) in &buckets_info {
        let is_selected = *idx == app.selected;

        lines.push(Line::raw(""));

        let header = format!(
            "  {} {} {}",
            bucket.emoji(),
            bucket.mode_description(),
            bucket.label()
        );
        let header_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        };
        lines.push(Line::styled(header, header_style));

        let entry_style = if is_selected {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        match entry_opt {
            Some(entry) => {
                let marker = if is_selected { "  ► " } else { "    " };
                lines.push(entry_name_line(entry, marker, entry_style));
                lines.push(entry_detail_line(entry, entry_style));
            }
            None => {
                lines.push(Line::styled(
                    "    (no restaurant found in this range)".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
    }
    lines.push(Line::raw(""));

    f.render_widget(Paragraph::new(lines), chunks[0]);

    let footer = Line::styled(
        "  ↑↓/jk Navigate   r Re-roll slot   R Re-roll all   Tab Browse   e Explore   Enter Accept   q Quit",
        Style::default().fg(Color::DarkGray),
    );
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

pub fn handle_key(app: &mut App, key: KeyCode) -> Action {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Enter => Action::Accept,
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            Action::Nothing
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
            Action::Nothing
        }
        KeyCode::Char('r') => {
            app.reroll_selected();
            Action::Nothing
        }
        KeyCode::Char('R') => {
            app.reroll_all();
            Action::Nothing
        }
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            let bucket_idx = app.selected;
            let entries = match bucket_idx {
                0 => app.buckets.near.as_slice(),
                1 => app.buckets.mid.as_slice(),
                _ => app.buckets.far.as_slice(),
            };
            let initial_cursor = app.slots[bucket_idx]
                .as_ref()
                .and_then(|pick| {
                    entries
                        .iter()
                        .position(|e| e.restaurant == pick.restaurant)
                })
                .unwrap_or(0);
            Action::SwitchToBrowse(BrowseState::new(bucket_idx, initial_cursor))
        }
        KeyCode::Char('e') => {
            let active_bucket = app.selected;
            let entries = match active_bucket {
                0 => app.buckets.near.as_slice(),
                1 => app.buckets.mid.as_slice(),
                _ => app.buckets.far.as_slice(),
            };
            Action::SwitchToExplorer(ExplorerState::new(active_bucket, entries))
        }
        _ => Action::Nothing,
    }
}

#[cfg(test)]
mod tests {
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
                address: name.into(),
                location: None,
            },
            bucket: Bucket::Near,
            best_secs: 720,
            best_mode: TravelMode::Transit,
            cuisines: cuisine.map(|c| vec![c.to_string()]).unwrap_or_default(),
        }
    }

    fn make_buckets() -> Buckets {
        Buckets {
            near: vec![make_entry("Hà", Some("vietnamese"))],
            mid: vec![make_entry("Nouveau Palais", Some("diner"))],
            far: vec![make_entry("Joe Beef", None)],
        }
    }

    #[test]
    fn render_does_not_panic() {
        let buckets = make_buckets();
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: Some(buckets.mid[0].clone()),
            far: Some(buckets.far[0].clone()),
        };
        let mut app = App::new(&buckets, initial);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app, f.area())).unwrap();

        app.move_down();
        terminal.draw(|f| draw(f, &app, f.area())).unwrap();
        app.move_down();
        terminal.draw(|f| draw(f, &app, f.area())).unwrap();
    }

    #[test]
    fn render_with_empty_buckets_does_not_panic() {
        let buckets = Buckets {
            near: vec![],
            mid: vec![],
            far: vec![],
        };
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let app = App::new(&buckets, initial);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app, f.area())).unwrap();
    }

    #[test]
    fn navigation_clamps_to_bounds() {
        let buckets = make_buckets();
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: None,
            far: None,
        };
        let mut app = App::new(&buckets, initial);
        assert_eq!(app.selected, 0);

        app.move_up();
        assert_eq!(app.selected, 0);

        app.move_down();
        app.move_down();
        assert_eq!(app.selected, 2);

        app.move_down();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn handle_key_tab_enters_browse() {
        let buckets = make_buckets();
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: None,
            far: None,
        };
        let mut app = App::new(&buckets, initial);
        let action = handle_key(&mut app, KeyCode::Tab);
        assert!(matches!(action, Action::SwitchToBrowse(_)));
    }

    #[test]
    fn handle_key_e_enters_explorer() {
        let buckets = make_buckets();
        let initial = Selection {
            near: Some(buckets.near[0].clone()),
            mid: None,
            far: None,
        };
        let mut app = App::new(&buckets, initial);
        let action = handle_key(&mut app, KeyCode::Char('e'));
        assert!(matches!(action, Action::SwitchToExplorer(_)));
    }

    #[test]
    fn handle_key_q_quits() {
        let buckets = make_buckets();
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let mut app = App::new(&buckets, initial);
        assert!(matches!(handle_key(&mut app, KeyCode::Char('q')), Action::Quit));
    }

    #[test]
    fn handle_key_enter_accepts() {
        let buckets = make_buckets();
        let initial = Selection {
            near: None,
            mid: None,
            far: None,
        };
        let mut app = App::new(&buckets, initial);
        assert!(matches!(handle_key(&mut app, KeyCode::Enter), Action::Accept));
    }
}
