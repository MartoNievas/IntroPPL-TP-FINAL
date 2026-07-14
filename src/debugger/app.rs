/*

Debugger application state: holds the currently-paused effect, the event
log shown in the history panel, and the snapshot stack used for read-only
"back" navigation. Owns the RNG used to resolve `sample` sites in step
mode (single draw) and continue mode (auto-run to the next
breakpoint/Done).

*/

use std::collections::HashSet;

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::interpreter::{Addr, Machine, Msg, resume, send};
use crate::parser::distribution::Distribution;
use crate::parser::value::RVal;

use ratatui::Terminal;

use super::event::{Command, next_command};
use super::render::{draw};
//use super::render::draw;

// What kind of probabilistic effect we're currently paused on, plus the
// data needed to render it and to decide how to resume. The `Distribution`
// itself (not just its label) is kept alive here because `step()` needs
// it to actually draw a sample.
pub enum PausedAt {
    Sample {
        addr: Addr,
        dist: Distribution,
        machine: Machine,
    },
    Factor {
        addr: Addr,
        log_prob: f64,
        machine: Machine,
    },
    Observe {
        addr: Addr,
        dist: Distribution,
        value: RVal,
        log_prob: f64,
        machine: Machine,
    },
    Done {
        value: RVal,
        log_w: f64,
    },
}

impl PausedAt {
    fn current_low_w(&self) -> f64 {
        match self {
            PausedAt::Sample { machine, .. } => machine.log_w,
            PausedAt::Factor { machine, .. } => machine.log_w,
            PausedAt::Observe { machine, .. } => machine.log_w,
            PausedAt::Done { log_w, .. } => *log_w,
        }
    }
}

// One entry in the scrollback history panel.
#[derive(Clone)]
pub struct LogEntry {
    pub addr_label: String,
    pub kind: &'static str, // "sample" | "observe"
    pub detail: String,
    pub log_w_after: f64,
}

// Cheap, renderable snapshot of a past PausedAt (no live Machine kept, so
// history entries are read-only -- see the module-level limitation note
// in mod.rs about not capturing RNG state).
#[derive(Clone)]
pub enum PausedAtSnapshot {
    Sample {
        addr: Addr,
        dist_name: String,
        log_w: f64,
    },
    Observe {
        addr: Addr,
        dist_name: String,
        value: String,
        log_prob: f64,
        log_w: f64,
    },
    Factor {
        addr: Addr,
        log_prob: f64,
        log_w: f64,
    },
    Done {
        value: String,
        log_w: f64,
    },
}

pub struct DebuggerApp {
    paused: PausedAt,
    log: Vec<LogEntry>,
    history: Vec<PausedAtSnapshot>,
    history_cursor: Option<usize>,
    breakpoints: HashSet<Addr>,
    rng: StdRng,
    should_quit: bool,
}

impl DebuggerApp {
    pub fn new(machine: Machine) -> Self {
        let paused = Self::advance_to_next_pause(machine).unwrap_or_else(|e| PausedAt::Done {
            value: RVal::Str(format!("Error: {e}")),
            log_w: 0.0,
        });

        DebuggerApp {
            paused,
            log: Vec::new(),
            history: Vec::new(),
            history_cursor: None,
            breakpoints: HashSet::new(),
            rng: StdRng::seed_from_u64(42),
            should_quit: false,
        }
    }

    // Calls `resume()` once and classifies the resulting Msg. This is the
    // only place that talks to the interpreter directly.
    fn advance_to_next_pause(machine: Machine) -> Result<PausedAt, String> {
        match resume(machine)? {
            Msg::Sample(addr, dist, m) => Ok(PausedAt::Sample {
                addr,
                dist,
                machine: m,
            }),
            Msg::Observe(addr, dist, y, m) => {
                let log_prob = dist.log_prob(&y);
                Ok(PausedAt::Observe {
                    addr,
                    dist,
                    value: y,
                    log_prob,
                    machine: m,
                })
            }
            Msg::Factor(addr, log_prob, m) => Ok(PausedAt::Factor {
                addr,
                log_prob,
                machine: m,
            }),
            Msg::Done(value, m) => Ok(PausedAt::Done {
                value,
                log_w: m.log_w,
            }),
        }
    }

    pub fn run_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), String> {
        loop {
            terminal
                .draw(|f| draw(f,self))
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
                self.step()?;
                let hit_breakpoint = matches!(
                    &self.paused,
                    PausedAt::Sample { addr, .. } | PausedAt::Observe { addr, .. }
                        if self.breakpoints.contains(addr)
                );
                if hit_breakpoint || matches!(self.paused, PausedAt::Done { .. }) {
                    break;
                }
            },
 
            Command::ToggleBreakpoint => {
                if let PausedAt::Sample { addr, .. } | PausedAt::Observe { addr, .. } = &self.paused {
                    let addr = addr.clone();
                    if !self.breakpoints.insert(addr.clone()) {
                        self.breakpoints.remove(&addr);
                    }
                }
            }
 
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

    // Advances the machine by exactly one probabilistic effect: resolves
    // the current pause point (sampling from the prior for `sample`,
    // re-injecting the fixed value for `observe`), then calls resume()
    // again to find the next one. No-op if already at `Done`.
    fn step(&mut self) -> Result<(), String> {
        self.push_history_snapshot();

        let owned = std::mem::replace(
            &mut self.paused,
            PausedAt::Done { value: RVal::Nil, log_w: 0.0 },
        );

        self.paused = match owned {
            PausedAt::Sample { addr, dist, mut machine } => {
                let x = dist.sample(&mut self.rng);
                let log_prob = dist.log_prob(&x);
                send(&mut machine, x.clone());
                let next = Self::advance_to_next_pause(machine)?;
                self.log.push(LogEntry {
                    addr_label: addr.join("/"),
                    kind: "sample",
                    detail: format!("{} -> {x} (log_prob {log_prob:.4})", dist.name()),
                    log_w_after: next.current_low_w(),
                });
                next
            }
            
            PausedAt::Observe { addr, dist, value, log_prob, mut machine } => {
                send(&mut machine, value.clone());
                let next = Self::advance_to_next_pause(machine)?;
                self.log.push(LogEntry {
                    addr_label: addr.join("/"),
                    kind: "observe",
                    detail: format!("{} @ {value} (log_prob {log_prob:.4})", dist.name()),
                    log_w_after: next.current_low_w(),
                });
                next
            }

            PausedAt::Factor { addr, log_prob, machine } => {
                
                let next = Self::advance_to_next_pause(machine)?;
                self.log.push(LogEntry {
                    addr_label: addr.join("/"),
                    kind: "factor",
                    detail: format!("log_w updated by {log_prob:.4}"),
                    log_w_after: next.current_low_w(),
                });
                next
            },

            done @ PausedAt::Done {..} => done,



        };

        Ok(())
    }


    fn push_history_snapshot(&mut self) {
        let snapshot = match &self.paused {
            PausedAt::Sample { addr, dist, machine } => PausedAtSnapshot::Sample {
                addr: addr.clone(),
                dist_name: dist.name().to_string(),
                log_w: machine.log_w,
            },

            PausedAt::Observe { addr, dist, value, log_prob, machine } => PausedAtSnapshot::Observe { addr: addr.clone(), dist_name: dist.name().to_string(), value: value.to_string(), log_prob: *log_prob, log_w: machine.log_w },
        
            PausedAt::Factor { addr, log_prob, machine } => PausedAtSnapshot::Factor { addr: addr.clone(), log_prob: *log_prob, log_w: machine.log_w },

            PausedAt::Done { value, log_w } => PausedAtSnapshot::Done { value: value.to_string(), log_w: *log_w },
        };
        self.history.push(snapshot);
    }


    // -- accessors used by render.rs --
    pub fn paused(&self) -> &PausedAt {
        &self.paused
    }
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }
    pub fn breakpoints(&self) -> &HashSet<Addr> {
        &self.breakpoints
    }
    pub fn history(&self) -> &[PausedAtSnapshot] {
        &self.history
    }
    pub fn viewing_history(&self) -> Option<usize> {
        self.history_cursor
    }
}
