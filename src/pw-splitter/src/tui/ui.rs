use crate::tui::app::{App, AppState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status
            Constraint::Length(2), // Help
        ])
        .split(frame.area());

    draw_title(frame, chunks[0], app);
    draw_main_content(frame, chunks[1], app);
    draw_status(frame, chunks[2], app);
    draw_help(frame, chunks[3], app);
}

fn draw_title(frame: &mut Frame, area: Rect, app: &App) {
    let title = match &app.state {
        AppState::SelectSource => "Select Audio Source",
        AppState::SelectDestination => "Select Recording Destination",
        AppState::Confirm => "Confirm Split Configuration",
        AppState::Active => "Split Active",
        AppState::Error(_) => "Error",
        AppState::Done => "Done",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" pw-splitter ");

    let paragraph = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan))
        .block(block);

    frame.render_widget(paragraph, area);
}

fn draw_main_content(frame: &mut Frame, area: Rect, app: &App) {
    match &app.state {
        AppState::SelectSource => draw_source_list(frame, area, app),
        AppState::SelectDestination => draw_destination_list(frame, area, app),
        AppState::Confirm => draw_confirm(frame, area, app),
        AppState::Active => draw_active(frame, area, app),
        AppState::Error(msg) => draw_error(frame, area, msg),
        AppState::Done => draw_done(frame, area),
    }
}

fn draw_source_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            let style = if i == app.selected_source_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if i == app.selected_source_idx {
                "> "
            } else {
                "  "
            };

            ListItem::new(format!("{}{}", prefix, source.display_name())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Audio Sources (applications producing audio) "),
    );

    frame.render_widget(list, area);
}

fn draw_destination_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .destinations
        .iter()
        .enumerate()
        .map(|(i, dest)| {
            let style = if i == app.selected_dest_idx {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if i == app.selected_dest_idx {
                "> "
            } else {
                "  "
            };

            ListItem::new(format!("{}{}", prefix, dest.display_name())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Recording Destinations (applications capturing audio) "),
    );

    frame.render_widget(list, area);
}

fn draw_confirm(frame: &mut Frame, area: Rect, app: &App) {
    let source_name = app
        .selected_source
        .as_ref()
        .map(|s| s.display_name())
        .unwrap_or_else(|| "None".to_string());

    let dest_name = app
        .selected_dest
        .as_ref()
        .map(|d| d.display_name())
        .unwrap_or_else(|| "None".to_string());

    let original_output = if app.source_connections.is_empty() {
        "No active connection (will use default output)".to_string()
    } else {
        app.source_connections
            .iter()
            .map(|c| c.target_node_name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Source: "),
            Span::styled(&source_name, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Recording Destination: "),
            Span::styled(&dest_name, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Original Output: "),
            Span::styled(&original_output, Style::default().fg(Color::Blue)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from("  Routing after split:"),
        Line::from(""),
        Line::from(format!("    [{}]", source_name)),
        Line::from("        |"),
        Line::from("        +---> [To Recording] ---> [OBS - full volume]"),
        Line::from("        |"),
        Line::from("        '---> [To Local] ---> [Speakers - adjustable]"),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Configuration "),
    );

    frame.render_widget(paragraph, area);
}

fn draw_active(frame: &mut Frame, area: Rect, app: &App) {
    let state = match &app.active_split {
        Some(s) => s,
        None => {
            let paragraph = Paragraph::new("No active split").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Active Split "),
            );
            frame.render_widget(paragraph, area);
            return;
        }
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  SPLIT ACTIVE",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(format!("  Source: {}", state.source_application_name)),
        Line::from(format!(
            "  Recording to: {}",
            state.recording_dest_application_name
        )),
        Line::from(format!(
            "  Local output: {}",
            state.original_output_node_name
        )),
        Line::from(""),
        Line::from("  Routing:"),
        Line::from(format!("    [{}]", state.source_application_name)),
        Line::from("        |"),
        Line::from("        +---> [To Recording] ---> [OBS] (FULL VOLUME)"),
        Line::from("        |"),
        Line::from("        '---> [To Local] ---> [Speakers] (ADJUSTABLE)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Adjust local volume in pwvucontrol",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(format!("  Look for: \"{}\"", state.local_loopback_name)),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Split "),
    );

    frame.render_widget(paragraph, area);
}

fn draw_error(frame: &mut Frame, area: Rect, message: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ERROR: ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(format!("  {}", message)),
    ];

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Error "));

    frame.render_widget(paragraph, area);
}

fn draw_done(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Split stopped successfully",
            Style::default().fg(Color::Green),
        )]),
        Line::from(""),
        Line::from("  Original connections have been restored."),
        Line::from(""),
        Line::from("  Press 'q' to quit or 'r' to create a new split."),
    ];

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Done "));

    frame.render_widget(paragraph, area);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let style = if app.status_message.contains("Error") || app.status_message.contains("Failed") {
        Style::default().fg(Color::Red)
    } else if app.status_message.contains("Warning") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let paragraph = Paragraph::new(app.status_message.as_str())
        .style(style)
        .block(Block::default().borders(Borders::ALL).title(" Status "));

    frame.render_widget(paragraph, area);
}

fn draw_help(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = match &app.state {
        AppState::SelectSource | AppState::SelectDestination => {
            "↑/↓: Navigate | Enter: Select | r: Refresh | q: Quit"
        }
        AppState::Confirm => "Enter: Confirm | Esc: Back | q: Quit",
        AppState::Active => "Enter: Stop Split | q: Quit (keeps split running)",
        AppState::Error(_) => "Esc: Back | q: Quit",
        AppState::Done => "r: New Split | q: Quit",
    };

    let paragraph = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}
