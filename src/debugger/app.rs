/*

Debugger application state: owns the algorithm-specific `Engine`, the
event log shown in the history panel, and a stack of rendered snapshots
used for read-only "back" navigation. Owns the RNG used to drive
whichever inference algorithm is active (single draw in step mode, and
repeated steps in continue mode).

*/

use std::collections::HashSet;

use rand::SeedableRng;
use rand::rngs::StdRng;

use ratatui::text::Line;
use ratatui::Terminal;

use crate::cli::Algorithm;
use crate::interpreter::Addr;

use super::engine::{Engine, StepReport};
use super::event::{Command, next_command};
use super::render::draw;

pub struct DebuggerApp {
    engine: Engine,
    program: String,
    log: Vec<StepReport>,
    history: Vec<Vec<Line<'static>>>,
    history_cursor: Option<usize>,
    breakpoints: HashSet<Addr>,
    rng: StdRng,
    should_quit: bool,
}

impl DebuggerApp {
    pub fn new(program: &str, algorithm: Algorithm) -> Result<Self, String> {
        let mut rng = StdRng::seed_from_u64(42);
        let engine = Engine::new(program, algorithm, &mut rng)?;

        Ok(DebuggerApp {
            engine,
            program: program.trim().to_string(),
            log: Vec::new(),
            history: Vec::new(),
            history_cursor: None,
            breakpoints: HashSet::new(),
            rng,
            should_quit: false,
        })
    }

    pub fn run_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), String> {
        loop {
            terminal
                .draw(|f| draw(f, self))
                .map_err(|e| format!("Debugger render error: {e}"))?;

            if self.should_quit {
                return Ok(());
            }

            if let Some(cmd) = next_command().map_err(|e| format!("Debugger input error: {e}"))? {
                self.handle_command(cmd)?;
            }
        }
    }

    fn handle_command(&mut self, cmd: Command) -> Result<(), String> {
        // Any command other than history navigation returns you to the
        // live pause point.
        if self.history_cursor.is_some() && !matches!(cmd, Command::Back | Command::Forward | Command::Quit) {
            self.history_cursor = None;
        }

        match cmd {
            Command::Quit => self.should_quit = true,

            Command::Step => self.step()?,

            Command::Continue => loop {
                if self.engine.is_finished() {
                    break;
                }
                self.step()?;
                let hit_breakpoint = self
                    .engine
                    .current_breakpoint_addr()
                    .is_some_and(|addr| self.breakpoints.contains(addr));
                if hit_breakpoint || self.engine.is_finished() {
                    break;
                }
            },

            Command::ToggleBreakpoint => {
                if let Some(addr) = self.engine.current_breakpoint_addr() {
                    let addr = addr.clone();
                    if !self.breakpoints.insert(addr.clone()) {
                        self.breakpoints.remove(&addr);
                    }
                }
            }

            Command::SelectPrev => self.engine.select_prev(),

            Command::SelectNext => self.engine.select_next(),

            Command::Back => {
                if !self.history.is_empty() {
                    self.history_cursor = Some(match self.history_cursor {
                        None => self.history.len() - 1,
                        Some(0) => 0,
                        Some(i) => i - 1,
                    });
                }
            }

            Command::Forward => {
                if let Some(i) = self.history_cursor {
                    self.history_cursor = if i + 1 < self.history.len() { Some(i + 1) } else { None };
                }
            }
        }
        Ok(())
    }

    // Advances the engine by exactly one of its own units of work. No-op
    // if already finished. Snapshots the current render before stepping,
    // so `Back` can show read-only history without needing to reconstruct
    // engine state.
    fn step(&mut self) -> Result<(), String> {
        if self.engine.is_finished() {
            return Ok(());
        }

        self.history.push(self.engine.render_current());

        let report = self.engine.step(&mut self.rng)?;
        self.log.push(report);

        Ok(())
    }

    // -- accessors used by render.rs --
    pub fn program(&self) -> &str {
        &self.program
    }
    pub fn current_lines(&self) -> Vec<Line<'static>> {
        match self.history_cursor {
            Some(i) => self.history[i].clone(),
            None => self.engine.render_current(),
        }
    }
    pub fn log(&self) -> &[StepReport] {
        &self.log
    }
    pub fn breakpoints(&self) -> &HashSet<Addr> {
        &self.breakpoints
    }
    pub fn engine_help_hint(&self) -> &'static str {
        self.engine.help_hint()
    }
    pub fn viewing_history(&self) -> Option<usize> {
        self.history_cursor
    }
}