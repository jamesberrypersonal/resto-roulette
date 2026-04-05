use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use crate::bucket::{Bucket, BucketEntry, Buckets};
use crate::config::OutputFormat;
use crate::display;
use crate::picker::{self, Selection};
use crate::routing::models::TravelMode;

struct App<'a> {
    buckets: &'a Buckets,
    slots: [Option<BucketEntry>; 3],
    selected: usize,
}

impl<'a> App<'a> {
    fn new(buckets: &'a Buckets, initial: Selection) -> Self {
        Self {
            buckets,
            slots: [initial.near, initial.mid, initial.far],
            selected: 0,
        }
    }

    fn reroll_selected(&mut self) {
        let candidates = match self.selected {
            0 => &self.buckets.near,
            1 => &self.buckets.mid,
            _ => &self.buckets.far,
        };
        self.slots[self.selected] = picker::pick_one_random(candidates);
    }

    fn reroll_all(&mut self) {
        self.slots[0] = picker::pick_one_random(&self.buckets.near);
        self.slots[1] = picker::pick_one_random(&self.buckets.mid);
        self.slots[2] = picker::pick_one_random(&self.buckets.far);
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(2);
    }

    fn to_selection(&self) -> Selection {
        Selection {
            near: self.slots[0].clone(),
            mid: self.slots[1].clone(),
            far: self.slots[2].clone(),
        }
    }
}

/// Run the interactive TUI. On accept, prints the final selection to stdout.
/// On quit, exits silently. Falls back gracefully on terminal errors.
pub fn run(buckets: &Buckets, initial: Selection) -> anyhow::Result<()> {
    let mut app = App::new(buckets, initial);

    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    let result = run_loop(&mut terminal, &mut app);

    // Always restore the terminal, even on error.
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    if let Some(selection) = result? {
        display::render(&selection, OutputFormat::Pretty);
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> anyhow::Result<Option<Selection>> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => return Ok(Some(app.to_selection())),
                    KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                    KeyCode::Char('r') => app.reroll_selected(),
                    KeyCode::Char('R') => app.reroll_all(),
                    _ => {}
                }
            }
        }
    }
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    let outer_block = Block::default()
        .title(" resto-roulette ")
        .borders(Borders::ALL);
    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    // Build slot lines
    let buckets_info = [
        (Bucket::Near, &app.slots[0], 0usize),
        (Bucket::Mid, &app.slots[1], 1),
        (Bucket::Far, &app.slots[2], 2),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (bucket, entry_opt, idx) in &buckets_info {
        let is_selected = *idx == app.selected;

        lines.push(Line::raw(""));

        // Bucket header
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
                let has_real_address = entry.restaurant.address != entry.restaurant.name;
                let cuisine = entry.cuisines.first().map(|c| capitalize(c));
                let parens = match (cuisine, has_real_address) {
                    (Some(c), true) => Some(format!("{} · {}", c, entry.restaurant.address)),
                    (Some(c), false) => Some(c),
                    (None, true) => Some(entry.restaurant.address.clone()),
                    (None, false) => None,
                };
                let name_line = match parens {
                    Some(p) => format!("{}{} ({})", marker, entry.restaurant.name, p),
                    None => format!("{}{}", marker, entry.restaurant.name),
                };
                lines.push(Line::styled(name_line, entry_style));
                lines.push(Line::styled(
                    format!("    {}", format_duration(entry.best_secs, &entry.best_mode)),
                    entry_style,
                ));
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

    // Footer
    let footer = Line::styled(
        "  ↑↓/jk Navigate   r Re-roll slot   R Re-roll all   Enter Accept   q Quit",
        Style::default().fg(Color::DarkGray),
    );
    f.render_widget(Paragraph::new(footer), chunks[1]);
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn format_duration(secs: u32, mode: &TravelMode) -> String {
    let mins = (secs + 30) / 60;
    let time = if mins < 60 {
        format!("~{} min", mins)
    } else {
        let h = mins / 60;
        let m = mins % 60;
        if m == 0 {
            format!("~{} h", h)
        } else {
            format!("~{} h {} min", h, m)
        }
    };
    format!("{} by {}", time, mode.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::Bucket;
    use crate::routing::models::TravelMode;
    use crate::Restaurant;
    use ratatui::backend::TestBackend;

    fn make_entry(name: &str, cuisine: Option<&str>) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: name.into(), // shared-list style: address == name
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
        terminal.draw(|f| draw(f, &app)).unwrap();

        // Navigate and re-render
        app.move_down();
        terminal.draw(|f| draw(f, &app)).unwrap();
        app.move_down();
        terminal.draw(|f| draw(f, &app)).unwrap();
    }

    #[test]
    fn render_with_empty_buckets_does_not_panic() {
        let buckets = Buckets {
            near: vec![],
            mid: vec![],
            far: vec![],
        };
        let initial = Selection { near: None, mid: None, far: None };
        let app = App::new(&buckets, initial);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap();
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

        app.move_up(); // already at 0
        assert_eq!(app.selected, 0);

        app.move_down();
        app.move_down();
        assert_eq!(app.selected, 2);

        app.move_down(); // already at 2
        assert_eq!(app.selected, 2);
    }
}
