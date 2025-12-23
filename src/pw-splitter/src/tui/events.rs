use crate::tui::app::{App, AppState};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

/// Handle input events
/// Returns true if the app should continue running
pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    // Poll for events with a timeout (allows for periodic checks like loopback monitoring)
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            // Only handle key press events (not release)
            if key.kind != KeyEventKind::Press {
                return Ok(!app.should_quit);
            }

            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
                }
                KeyCode::Enter => {
                    app.confirm_selection();
                }
                KeyCode::Esc => {
                    app.go_back();
                }
                KeyCode::Char('r') => {
                    // Refresh or restart
                    match &app.state {
                        AppState::SelectSource | AppState::SelectDestination => {
                            if let Err(e) = app.refresh() {
                                app.status_message = format!("Refresh failed: {}", e);
                            } else {
                                app.status_message = "Refreshed".to_string();
                            }
                        }
                        AppState::Done | AppState::Error(_) => {
                            // Reset to start a new split
                            if let Ok(new_app) = App::new() {
                                *app = new_app;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    } else {
        // No event - do periodic checks
        if app.state == AppState::Active {
            app.check_and_restart_loopbacks();
        }
    }

    Ok(!app.should_quit)
}
