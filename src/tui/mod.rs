// wstrace - Terminal Event Loop & Crossterm Engine

pub mod app;
pub mod clipboard;
pub mod ui;

use std::io::{stdout, Result};
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::event::TraceEvent;
use app::App;

pub fn run_tui(mut app: App, rx: Receiver<TraceEvent>) -> Result<Vec<TraceEvent>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let res = run_loop(&mut terminal, &mut app, rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res?;
    Ok(app.events)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    rx: Receiver<TraceEvent>,
) -> Result<()> {
    while !app.should_quit {
        // Drain incoming event bus in non-blocking loop
        while let Ok(ev) = rx.try_recv() {
            app.push_event(ev);
        }

        terminal.draw(|f| ui::render(f, app))?;

        // 60 FPS polling window (~16ms)
        if event::poll(Duration::from_millis(16))? {
            if let CEvent::Key(key) = event::read()? {
                // Ignore key release events on Windows console to prevent double-stepping
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Windows-native Process Monitor Control key combinations
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match key.code {
                        KeyCode::Char('c') => {
                            app.copy_selected_to_clipboard();
                        }
                        KeyCode::Char('a') => {
                            app.copy_all_to_clipboard();
                        }
                        KeyCode::Char('e') => {
                            app.toggle_pause();
                        }
                        KeyCode::Char('x') => {
                            app.clear_events();
                        }
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                            break;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Standard navigational shortcuts
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char(' ') => app.toggle_pause(),
                    KeyCode::Char('f') => app.toggle_failures_only(),
                    KeyCode::Tab => app.cycle_category(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
