/*

Likelihood Weighting debug engine: the one algorithm whose execution
model is already "a single trace that pauses at each probabilistic
effect", so this is a near-direct port of the debugger's original
(LW-only) implementation. The one behavioral fix here relative to that
original version: `Observe` and `Factor` now actually add their
log-probability/weight to `machine.log_w` before continuing, matching
what `inference::lw::run_lw` does -- previously "log_w so far" never
moved because that accumulation step was missing entirely.

*/

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use rand::Rng;

use crate::debugger::engine::StepReport;
use crate::interpreter::{initial_machine, resume, send, Addr, Machine, Msg};
use crate::parser::distribution::Distribution;
use crate::parser::value::RVal;

pub enum LwPaused {
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

pub struct LwEngine {
    paused: LwPaused,
}

impl LwEngine {
    pub fn new(program: &str) -> Result<Self, String> {
        let machine = initial_machine(program)?;
        let paused = Self::advance_to_next_pause(machine)?;
        Ok(LwEngine { paused })
    }

    fn advance_to_next_pause(machine: Machine) -> Result<LwPaused, String> {
        match resume(machine)? {
            Msg::Sample(addr, dist, m) => Ok(LwPaused::Sample {
                addr,
                dist,
                machine: m,
            }),
            Msg::Observe(addr, dist, y, m) => {
                let log_prob = dist.log_prob(&y);
                Ok(LwPaused::Observe {
                    addr,
                    dist,
                    value: y,
                    log_prob,
                    machine: m,
                })
            }
            Msg::Factor(addr, log_prob, m) => Ok(LwPaused::Factor {
                addr,
                log_prob,
                machine: m,
            }),
            Msg::Done(value, m) => Ok(LwPaused::Done {
                value,
                log_w: m.log_w,
            }),
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.paused, LwPaused::Done { .. })
    }

    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        match &self.paused {
            LwPaused::Sample { addr, .. }
            | LwPaused::Observe { addr, .. }
            | LwPaused::Factor { addr, .. } => Some(addr),
            LwPaused::Done { .. } => None,
        }
    }

    pub fn step<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<StepReport, String> {
        let owned = std::mem::replace(
            &mut self.paused,
            LwPaused::Done {
                value: RVal::Nil,
                log_w: 0.0,
            },
        );

        let (next, report) = match owned {
            LwPaused::Sample { addr, dist, mut machine } => {
                let x = dist.sample(rng);
                let log_prob = dist.log_prob(&x);
                send(&mut machine, x.clone());
                let next = Self::advance_to_next_pause(machine)?;
                let report = StepReport {
                    addr_label: addr.join("/"),
                    kind: "sample",
                    detail: format!("{} -> {x} (log_prob {log_prob:.4})", dist.name()),
                    metric_label: "log_w",
                    metric_value: next_log_w(&next),
                };
                (next, report)
            }

            LwPaused::Observe { addr, dist, value, log_prob, mut machine } => {
                machine.log_w += log_prob;
                send(&mut machine, value.clone());
                let next = Self::advance_to_next_pause(machine)?;
                let report = StepReport {
                    addr_label: addr.join("/"),
                    kind: "observe",
                    detail: format!("{} @ {value} (log_prob {log_prob:.4})", dist.name()),
                    metric_label: "log_w",
                    metric_value: next_log_w(&next),
                };
                (next, report)
            }

            LwPaused::Factor { addr, log_prob, mut machine } => {
                machine.log_w += log_prob;
                let next = Self::advance_to_next_pause(machine)?;
                let report = StepReport {
                    addr_label: addr.join("/"),
                    kind: "factor",
                    detail: format!("log_w updated by {log_prob:.4}"),
                    metric_label: "log_w",
                    metric_value: next_log_w(&next),
                };
                (next, report)
            }

            done @ LwPaused::Done { .. } => {
                let log_w = if let LwPaused::Done { log_w, .. } = &done { *log_w } else { 0.0 };
                let report = StepReport {
                    addr_label: String::new(),
                    kind: "done",
                    detail: "program already finished".into(),
                    metric_label: "log_w",
                    metric_value: log_w,
                };
                (done, report)
            }
        };

        self.paused = next;
        Ok(report)
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        match &self.paused {
            LwPaused::Sample { addr, dist, machine } => vec![
                Line::from(vec![
                    Span::styled(
                        "SAMPLE  ",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(addr.join("/")),
                ]),
                Line::from(format!("distribution: {}", dist.name())),
                Line::from(format!("log_w so far: {:.4}", machine.log_w)),
                Line::from(""),
                Line::from("[s] draw from prior and continue   [b] toggle breakpoint here"),
            ],
            LwPaused::Factor { addr, log_prob, machine } => vec![
                Line::from(vec![
                    Span::styled(
                        "FACTOR  ",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(addr.join("/")),
                ]),
                Line::from(format!("log-weight added: {log_prob:.4}")),
                Line::from(format!("log_w so far: {:.4}", machine.log_w)),
                Line::from(""),
                Line::from("[s] step   [c] continue   [b] toggle breakpoint here"),
            ],
            LwPaused::Observe { addr, dist, value, log_prob, machine } => vec![
                Line::from(vec![
                    Span::styled(
                        "OBSERVE ",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(addr.join("/")),
                ]),
                Line::from(format!("distribution: {}", dist.name())),
                Line::from(format!("observed value: {value}")),
                Line::from(format!("log_prob: {log_prob:.4}")),
                Line::from(format!("log_w so far: {:.4}", machine.log_w)),
                Line::from("[s] accept observed value and continue   [b] toggle breakpoint here"),
            ],
            LwPaused::Done { value, log_w } => vec![
                Line::from(Span::styled(
                    "DONE",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("result: {value}")),
                Line::from(format!("final log_w: {log_w:.4}")),
                Line::from(""),
                Line::from("[q] quit"),
            ],
        }
    }
}

fn next_log_w(paused: &LwPaused) -> f64 {
    match paused {
        LwPaused::Sample { machine, .. } => machine.log_w,
        LwPaused::Factor { machine, .. } => machine.log_w,
        LwPaused::Observe { machine, .. } => machine.log_w,
        LwPaused::Done { log_w, .. } => *log_w,
    }
}
