/*

Panel layout for the debugger TUI: a status header, a "current pause
point" panel (delegated to the active `Engine`, since each algorithm
renders a very different kind of state), a scrollback log of past steps,
and a help footer with the active keybindings. Pure rendering -- no state
mutation happens here, `app.rs` and the `engine` submodule own everything
this module reads.

*/

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::DebuggerApp;

pub fn draw(f: &mut Frame, app: &DebuggerApp) {
    let area = f.area();

    // Border + title take 2 rows, so line count + 2, clamped so a one-line
    // model doesn't waste space and a long one doesn't crowd out the log.
    let model_height = (app.program().lines().count() as u16 + 2).clamp(4, 10);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(model_height),
            Constraint::Length(14),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, chunks[0], app);
    draw_model(f, chunks[1], app);
    draw_current(f, chunks[2], app);
    draw_log(f, chunks[3], app);
    draw_help(f, chunks[4], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let title = if app.viewing_history().is_some() {
        " HOPPL Debugger -- VIEWING HISTORY (read-only) "
    } else {
        " HOPPL Debugger "
    };

    let style = if app.viewing_history().is_some() {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, style));

    f.render_widget(block, area);
}

fn draw_model(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let block = Block::default().borders(Borders::ALL).title(" Model ");
    let paragraph = Paragraph::new(app.program())
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_current(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let block = Block::default().borders(Borders::ALL).title(" Current ");
    let paragraph = Paragraph::new(app.current_lines())
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_log(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let items: Vec<ListItem> = app
        .log()
        .iter()
        .rev() // most recent first
        .map(|entry| {
            let kind_color = match entry.kind {
                "sample" | "branch" => Color::Green,
                "observe" | "factor" | "round" => Color::Magenta,
                "iteration" | "adam" => Color::Cyan,
                _ => Color::Gray,
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<9} ", entry.kind),
                    Style::default().fg(kind_color),
                ),
                Span::raw(format!("{:<24} ", entry.addr_label)),
                Span::raw(format!("{:<40} ", entry.detail)),
                Span::styled(
                    format!("{}={:.4}", entry.metric_label, entry.metric_value),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Event log ({} steps) ", app.log().len()));
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_help(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let breakpoints_count = app.breakpoints().len();
    let mut text = format!(
        "[s] step  [c] continue  [b] toggle breakpoint ({breakpoints_count} set)  [<-/->] browse history  [q] quit"
    );
    let hint = app.engine_help_hint();
    if !hint.is_empty() {
        text.push_str("  ");
        text.push_str(hint);
    }
    let block = Block::default().borders(Borders::ALL).title(" Controls ");
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
