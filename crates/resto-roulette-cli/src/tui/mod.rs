pub mod app;
pub mod browse_view;
pub mod explorer_view;
pub mod slots_view;
pub mod widgets;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::config::OutputFormat;
use crate::display;
use resto_roulette_core::bucket::Buckets;
use resto_roulette_core::picker::Selection;

use app::{Action, App, Mode, ModeKind};

/// Run the interactive TUI. On accept, prints the final selection to stdout.
/// On quit, exits silently. Falls back gracefully on terminal errors.
///
/// Set `explore` to `true` to launch directly into the full-screen explorer mode.
pub fn run(buckets: &Buckets, initial: Selection, explore: bool) -> anyhow::Result<()> {
    let mut app = if explore {
        App::new_in_explorer(buckets, initial)
    } else {
        App::new(buckets, initial)
    };

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
        terminal.draw(|f| {
            let area = f.area();
            match app.mode_kind() {
                ModeKind::Slots => slots_view::draw(f, app, area),
                ModeKind::Browse => browse_view::draw(f, app, area),
                ModeKind::Explorer => explorer_view::draw(f, app, area),
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let kind = app.mode_kind();
                let action: Action = match kind {
                    ModeKind::Slots => slots_view::handle_key(app, key.code),
                    ModeKind::Browse => browse_view::handle_key(app, key.code),
                    ModeKind::Explorer => explorer_view::handle_key(app, key.code),
                };

                match action {
                    Action::Nothing => {}
                    Action::SwitchToSlots => app.mode = Mode::Slots,
                    Action::SwitchToBrowse(state) => app.mode = Mode::Browse(state),
                    Action::SwitchToExplorer(state) => app.mode = Mode::Explorer(state),
                    Action::Accept => return Ok(Some(app.to_selection())),
                    Action::Quit => return Ok(None),
                }
            }
        }
    }
}
