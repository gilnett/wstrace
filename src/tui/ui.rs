// wstrace - Ratatui TUI Rendering Engine

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::event::EventCategory;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header / Telemetry statistics
            Constraint::Min(8),    // Event stream table
            Constraint::Length(6), // Detailed inspector panel
            Constraint::Length(1), // Keybindings footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0], app);
    render_events_table(frame, chunks[1], app);
    render_detail_pane(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let filtered_count = app.filtered_events().len();
    let total_count = app.events.len();

    let status_badge = if app.is_terminated {
        Span::styled(" [TERMINATED] ", Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD))
    } else if app.is_paused {
        Span::styled(" [PAUSED] ", Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [LIVE] ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
    };

    let cat_text = match app.filter_category {
        Some(c) => c.as_str(),
        None => "ALL",
    };

    let filter_badge = if app.filter_failures_only {
        Span::styled(" [FAILURES ONLY] ", Style::default().bg(Color::Red).fg(Color::White))
    } else {
        Span::styled(format!(" [CAT: {}] ", cat_text), Style::default().fg(Color::Cyan))
    };

    let target_tag = if app.events.iter().any(|e| e.operation == "ProcessHandOff") {
        " [Target App]"
    } else if app.events.iter().any(|e| e.operation == "LauncherAttach" || e.operation == "LauncherExit") {
        " [Launcher]"
    } else {
        ""
    };

    let header_line = Line::from(vec![
        Span::styled(" wstrace ", Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(" | Target: "),
        Span::styled(format!("{}{}", app.target_name, target_tag), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" (PID: {}) | Events: {}/{} |", app.target_pid, filtered_count, total_count)),
        status_badge,
        filter_badge,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Windows System & API Monitor ");

    let paragraph = Paragraph::new(header_line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_events_table(frame: &mut Frame, area: Rect, app: &mut App) {
    let filtered = app.filtered_events();
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, (_, ev))| {
            let cat_color = match ev.category {
                EventCategory::FileSystem => Color::Cyan,
                EventCategory::Registry => Color::Magenta,
                EventCategory::Network => Color::LightGreen,
                EventCategory::Process => Color::Yellow,
                EventCategory::Memory => Color::Blue,
                EventCategory::Error => Color::Red,
            };

            let res_color = if ev.is_failure {
                Color::Red
            } else {
                Color::Green
            };

            let is_selected = i == app.selected_index;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let op_cell = match ev.operation.as_str() {
                "LauncherAttach" | "LauncherExit" => {
                    Cell::from(Span::styled(ev.operation.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                }
                "ProcessHandOff" => {
                    Cell::from(Span::styled(ev.operation.as_str(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
                }
                _ => Cell::from(ev.operation.as_str()),
            };

            let details_cell = if ev.details.starts_with("[Launcher]") {
                Cell::from(Span::styled(ev.details.as_str(), Style::default().fg(Color::Rgb(255, 200, 100))))
            } else if ev.details.starts_with("[Target App]") {
                Cell::from(Span::styled(ev.details.as_str(), Style::default().fg(Color::Rgb(120, 230, 120))))
            } else {
                Cell::from(ev.details.as_str())
            };

            Row::new(vec![
                Cell::from(ev.timestamp.as_str()),
                Cell::from(Span::styled(ev.category.as_str(), Style::default().fg(cat_color))),
                op_cell,
                Cell::from(ev.target.as_str()),
                Cell::from(Span::styled(ev.result.as_str(), Style::default().fg(res_color))),
                details_cell,
            ])
            .style(style)
        })
        .collect();


    let widths = [
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(18),
        Constraint::Percentage(45),
        Constraint::Length(22),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Time", "Cat", "Operation", "Target / Path / Key", "Result", "Details"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(0),
        )
        .block(Block::default().borders(Borders::ALL).title(" Event Stream "));

    let mut state = TableState::default();
    if !filtered.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

fn render_detail_pane(frame: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_events();
    let detail_text = if let Some((_, ev)) = filtered.get(app.selected_index) {
        vec![
            Line::from(vec![
                Span::styled("Operation: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{} (TID: {})", ev.operation, ev.tid)),
                Span::raw(" | "),
                Span::styled("Result: ", Style::default().fg(Color::Yellow)),
                Span::styled(&ev.result, if ev.is_failure { Style::default().fg(Color::Red) } else { Style::default().fg(Color::Green) }),
            ]),
            Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::Cyan)),
                Span::raw(&ev.target),
            ]),
            Line::from(vec![
                Span::styled("Metadata: ", Style::default().fg(Color::Magenta)),
                Span::raw(&ev.details),
            ]),
        ]
    } else {
        vec![Line::from(Span::styled("No event selected.", Style::default().fg(Color::DarkGray)))]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Event Inspector ");
    let paragraph = Paragraph::new(detail_text).block(block);
    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut help_spans = vec![
        Span::styled("[Ctrl+C]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Copy  "),
        Span::styled("[Ctrl+A]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Copy All  "),
        Span::styled("[Ctrl+E/Space]", Style::default().fg(Color::Yellow)),
        Span::raw(" Pause  "),
        Span::styled("[Ctrl+X]", Style::default().fg(Color::Yellow)),
        Span::raw(" Clear  "),
        Span::styled("[Tab]", Style::default().fg(Color::Yellow)),
        Span::raw(" Category  "),
        Span::styled("[f]", Style::default().fg(Color::Yellow)),
        Span::raw(" Failures  "),
        Span::styled("[Up/Down]", Style::default().fg(Color::Yellow)),
        Span::raw(" Scroll  "),
        Span::styled("[q/Esc]", Style::default().fg(Color::Red)),
        Span::raw(" Quit"),
    ];

    if let Some(msg) = &app.status_message {
        help_spans.push(Span::raw("  |  "));
        help_spans.push(Span::styled(msg, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
    }

    let paragraph = Paragraph::new(Line::from(help_spans));
    frame.render_widget(paragraph, area);
}
