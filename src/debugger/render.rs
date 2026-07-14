/*

Panel layout for the debugger TUI: a status header, a "current pause point"
panel, a scrollback log of past sample/observe events, and a help footer
with the active keybindings. Pure rendering -- no state mutation happens
here, `app.rs` owns everything this module reads.

*/

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::app::{DebuggerApp, PausedAt, PausedAtSnapshot};

pub fn draw(f: &mut Frame, app: &DebuggerApp) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, chunks[0], app);
    draw_current(f, chunks[1], app);
    draw_log(f, chunks[2], app);
    draw_help(f, chunks[3], app);
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

fn draw_current(f: &mut Frame, area: Rect, app: &DebuggerApp) {}

fn render_live(paused: &PausedAt) -> Vec<Line<'static>> {
    match paused {
        PausedAt::Sample {
            addr,
            dist,
            machine,
        } => vec![
            Line::from(vec![
                Span::styled(
                    "SAMPLE  ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("distribution: {}", dist.name())),
            Line::from(format!("log_w so far: {:.4}", machine.log_w)),
            Line::from(""),
            Line::from("[s] draw from prior and continue   [b] toggle breakpoint here"),
        ],
        PausedAt::Factor {
            addr,
            log_prob,
            machine,
        } => vec![
            Line::from(vec![
                Span::styled(
                    "FACTOR  ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("log-weight added: {log_prob:.4}")),
            Line::from(format!("log_w so far: {:.4}", machine.log_w)),
            Line::from(""),
            Line::from("[c] continue   [b] toggle breakpoint here"),
        ],
        PausedAt::Observe {
            addr,
            dist,
            value,
            log_prob,
            machine,
        } => vec![
            Line::from(vec![
                Span::styled(
                    "OBSERVE ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("distribution: {}", dist.name())),
            Line::from(format!("observed value: {value}")),
            Line::from(format!("log_prob: {log_prob:.4}")),
            Line::from(format!("log_w so far: {:.4}", machine.log_w)),
        ],
        PausedAt::Done { value, log_w } => vec![
            Line::from(Span::styled(
                "DONE",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("result: {value}")),
            Line::from(format!("final log_w: {log_w:.4}")),
            Line::from(""),
            Line::from("[q] quit"),
        ],
    }
}

fn render_snapshot(snap: &PausedAtSnapshot) -> Vec<Line<'static>> {
    match snap {
        PausedAtSnapshot::Sample {
            addr,
            dist_name,
            log_w,
        } => vec![
            Line::from(vec![
                Span::styled("SAMPLE ", Style::default().fg(Color::Green)),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("dsitribution: {dist_name}")),
            Line::from(format!("log_w so far: {log_w:.4}")),
        ],

        PausedAtSnapshot::Observe {
            addr,
            dist_name,
            value,
            log_prob,
            log_w,
        } => vec![
            Line::from(vec![
                Span::styled("OBSERVE ", Style::default().fg(Color::Magenta)),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("distribution: {dist_name}")),
            Line::from(format!("observed value: {value}")),
            Line::from(format!("log_prob: {log_prob:.4}")),
            Line::from(format!("log_w so far: {log_w:.4}")),
        ],

        PausedAtSnapshot::Factor {
            addr,
            log_prob,
            log_w,
        } => vec![
            Line::from(vec![
                Span::styled("FACTOR ", Style::default().fg(Color::Magenta)),
                Span::raw(addr.join("/")),
            ]),
            Line::from(format!("log_prob: {log_prob:.4}")),
            Line::from(format!("log_w so far: {log_w:.4}")),
        ],

        PausedAtSnapshot::Done { value, log_w } => vec![
            Line::from(Span::styled("DONE ", Style::default().fg(Color::Blue))),
            Line::from(format!("result: {value}")),
            Line::from(format!("final log_w: {log_w:.4}")),
        ],
    }
}

fn draw_log(f: &mut Frame, area: Rect, app: &DebuggerApp) {
    let items: Vec<ListItem> = app
        .log()
        .iter()
        .rev() // most recent first
        .map(|entry| {
            let kind_color = if entry.kind == "sample" {
                Color::Green
            } else {
                Color::Magenta
            };
            let is_breakpoint = false; // breakpoints are on the *pending* address, not past log entries
            let _ = is_breakpoint;
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<7} ", entry.kind),
                    Style::default().fg(kind_color),
                ),
                Span::raw(format!("{:<24} ", entry.addr_label)),
                Span::raw(format!("{:<40} ", entry.detail)),
                Span::styled(
                    format!("log_w={:.4}", entry.log_w_after),
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
    let text = format!(
        "[s] step  [c] continue  [b] toggle breakpoint ({breakpoints_count} set)  [<-/->] browse history  [q] quit"
    );
    let block = Block::default().borders(Borders::ALL).title(" Controls ");
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
