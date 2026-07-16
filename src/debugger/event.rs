/*
 
Keyboard input handling for the debugger. Polls crossterm for key events
with a short timeout (so the UI can still redraw/animate even with no
input) and translates recognized keys into `Command`s that `app.rs` acts on.
 
*/

use std::time::Duration;

use::crossterm::event::{self, Event, KeyCode, KeyEventKind};

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Step,             // 's': advance exactly one unit of work
    Continue,         // 'c': auto-run until the next breakpoint or Done
    ToggleBreakpoint, // 'b': toggle a breakpoint at the current address
    SelectPrev,       // Up arrow: move the highlighted choice (Exact Enumeration only)
    SelectNext,       // Down arrow: move the highlighted choice (Exact Enumeration only)
    Back,             // Left arrow: view the previous pause point (read-only)
    Forward,          // Right arrow: move forward in history playback
    Quit,             // 'q' or Esc
}

const POLL_TIMEOUT: Duration = Duration::from_millis(150);

 
/// Polls for a key event within `POLL_TIMEOUT`. Returns `Ok(None)` if no
/// relevant key arrived in that window (the caller should just redraw and
/// poll again), so the UI stays responsive without busy-waiting on a
/// blocking read.
pub fn next_command() -> Result<Option<Command>, std::io::Error> {
    if !event::poll(POLL_TIMEOUT)? {
        return Ok(None);
    }

    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(None);
        }

        let cmd = match key.code {
            KeyCode::Char('s') => Command::Step,
            KeyCode::Char('c') => Command::Continue,
            KeyCode::Char('b') => Command::ToggleBreakpoint,
            KeyCode::Up => Command::SelectPrev,
            KeyCode::Down => Command::SelectNext,
            KeyCode::Left => Command::Back,
            KeyCode::Right => Command::Forward,
            KeyCode::Char('q') | KeyCode::Esc => Command::Quit,
            _ => return Ok(None),
        };
        return Ok(Some(cmd));
    }
    Ok(None)
}