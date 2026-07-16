/*

Exact Enumeration debug engine. Unlike LW, this isn't a single trace: at
every `sample` the real algorithm (`inference::exact_enumeration::enumerate_traces`)
forks one child machine per value in the distribution's finite support and
explores all of them via a stack. Here the user plays the role that stack
normally plays automatically: at each `sample` the panel shows the full
support (value + log-prob) and lets them pick which branch to descend
into with the arrow keys; the sibling branches are pushed onto `pending`
and stay there until their turn comes (via `step()` on a finished leaf,
which pops the next pending branch). `Observe`/`Factor` never branch, so
they're resolved automatically in between, exactly like
`enumerate_traces` does.

*/

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::debugger::engine::StepReport;
use crate::inference::defaults::ENUM_MAX_TRACES;
use crate::inference::exact_enumeration::posterior_table;
use crate::interpreter::{initial_machine, resume, send, Addr, Machine, Msg};
use crate::parser::distribution::Distribution;
use crate::parser::value::RVal;
use crate::stats::{as_f64, is_numeric};

pub enum EnumCurrent {
    // Paused at a `sample` with a user-selectable branch highlighted.
    Choice {
        addr: Addr,
        dist: Distribution,
        support: Vec<(RVal, f64)>,
        selected: usize,
        machine: Machine,
    },
    // A trace just finished; step() moves on to the next pending branch.
    Leaf { value: RVal, log_w: f64 },
    // No leaf pending and nothing left in `pending`: the whole tree has
    // been explored.
    Finished,
}

pub struct EnumEngine {
    current: EnumCurrent,
    pending: Vec<Machine>,
    finished: Vec<(RVal, f64)>,
    visited: usize,
    max_states: usize,
}

impl EnumEngine {
    pub fn new(program: &str) -> Result<Self, String> {
        let machine = initial_machine(program)?;
        let current = Self::advance_to_choice_or_leaf(machine)?;
        Ok(EnumEngine {
            current,
            pending: Vec::new(),
            finished: Vec::new(),
            visited: 0,
            max_states: ENUM_MAX_TRACES,
        })
    }

    // Runs resume() in a loop, auto-resolving Observe/Factor (no branching
    // there, only `sample` forks) until it hits a `sample` -- returned as
    // a Choice for the user to pick a branch from -- or the program
    // finishes (a Leaf).
    fn advance_to_choice_or_leaf(mut m: Machine) -> Result<EnumCurrent, String> {
        loop {
            match resume(m)? {
                Msg::Sample(addr, dist, m_sam) => {
                    let support = dist.finite_support()?;
                    return Ok(EnumCurrent::Choice {
                        addr,
                        dist,
                        support,
                        selected: 0,
                        machine: m_sam,
                    });
                }
                Msg::Observe(_addr, dist, y_obs, mut m_obs) => {
                    m_obs.log_w += dist.log_prob(&y_obs);
                    send(&mut m_obs, y_obs);
                    m = m_obs;
                }
                Msg::Factor(_addr, w, mut next_m) => {
                    next_m.log_w += w;
                    send(&mut next_m, RVal::Nil);
                    m = next_m;
                }
                Msg::Done(value, m_done) => {
                    return Ok(EnumCurrent::Leaf {
                        value,
                        log_w: m_done.log_w,
                    });
                }
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.current, EnumCurrent::Finished)
    }

    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        match &self.current {
            EnumCurrent::Choice { addr, .. } => Some(addr),
            _ => None,
        }
    }

    pub fn select_prev(&mut self) {
        if let EnumCurrent::Choice { support, selected, .. } = &mut self.current {
            *selected = if *selected == 0 { support.len() - 1 } else { *selected - 1 };
        }
    }

    pub fn select_next(&mut self) {
        if let EnumCurrent::Choice { support, selected, .. } = &mut self.current {
            *selected = (*selected + 1) % support.len();
        }
    }

    pub fn step(&mut self) -> Result<StepReport, String> {
        self.visited += 1;
        if self.visited > self.max_states {
            return Err(format!(
                "Exact Enumeration: exceeded the state limit ({})",
                self.max_states
            ));
        }

        let owned = std::mem::replace(&mut self.current, EnumCurrent::Finished);

        let (next, report) = match owned {
            EnumCurrent::Choice { addr, dist, support, selected, machine } => {
                let (chosen_val, chosen_lp) = support[selected].clone();

                for (i, (x, lp)) in support.iter().enumerate() {
                    if i == selected {
                        continue;
                    }
                    let mut sibling = machine.fork();
                    sibling.log_w += lp;
                    send(&mut sibling, x.clone());
                    self.pending.push(sibling);
                }

                let mut chosen = machine;
                chosen.log_w += chosen_lp;
                send(&mut chosen, chosen_val.clone());
                let next = Self::advance_to_choice_or_leaf(chosen)?;

                let report = StepReport {
                    addr_label: addr.join("/"),
                    kind: "branch",
                    detail: format!(
                        "{} -> {chosen_val} (log_prob {chosen_lp:.4}), {} sibling branch(es) queued",
                        dist.name(),
                        support.len() - 1
                    ),
                    metric_label: "log_w",
                    metric_value: current_log_w(&next),
                };
                (next, report)
            }

            EnumCurrent::Leaf { value, log_w } => {
                self.finished.push((value.clone(), log_w));

                let next = if let Some(sibling) = self.pending.pop() {
                    Self::advance_to_choice_or_leaf(sibling)?
                } else {
                    EnumCurrent::Finished
                };

                let report = StepReport {
                    addr_label: String::new(),
                    kind: "leaf",
                    detail: format!(
                        "trace finished: {value} (log_w {log_w:.4}), {} branch(es) left to explore",
                        self.pending.len()
                    ),
                    metric_label: "log_w",
                    metric_value: log_w,
                };
                (next, report)
            }

            EnumCurrent::Finished => {
                let report = StepReport {
                    addr_label: String::new(),
                    kind: "done",
                    detail: "exploration already complete".into(),
                    metric_label: "log_w",
                    metric_value: 0.0,
                };
                (EnumCurrent::Finished, report)
            }
        };

        self.current = next;
        Ok(report)
    }

    pub fn help_hint(&self) -> &'static str {
        if matches!(self.current, EnumCurrent::Choice { .. }) {
            "[up/down] select branch"
        } else {
            ""
        }
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        match &self.current {
            EnumCurrent::Choice { addr, dist, support, selected, machine } => {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            "SAMPLE (choose branch) ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(addr.join("/")),
                    ]),
                    Line::from(format!(
                        "distribution: {}   log_w so far: {:.4}",
                        dist.name(),
                        machine.log_w
                    )),
                ];
                for (i, (val, lp)) in support.iter().enumerate() {
                    let marker = if i == *selected { "> " } else { "  " };
                    let style = if i == *selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{marker}{val} (log_prob {lp:.4})"),
                        style,
                    )));
                }
                lines.push(Line::from(
                    "[up/down] select   [s] descend   [b] toggle breakpoint here",
                ));
                lines
            }
            EnumCurrent::Leaf { value, log_w } => vec![
                Line::from(Span::styled(
                    "LEAF",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("trace result: {value}   log_w: {log_w:.4}")),
                Line::from(""),
                Line::from("[s] explore next pending branch"),
            ],
            EnumCurrent::Finished => {
                let (pmf, log_z) = posterior_table(&self.finished);
                let mut lines = vec![
                    Line::from(Span::styled(
                        "DONE (all branches explored)",
                        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(format!(
                        "states explored: {}   log evidence (Z): {log_z:.4}",
                        self.finished.len()
                    )),
                ];
                if pmf.iter().all(|(v, _, _)| is_numeric(v)) {
                    let posterior_mean: f64 = pmf
                        .iter()
                        .map(|(v, prob, _)| as_f64(v).unwrap() * prob)
                        .sum();
                    lines.push(Line::from(format!(
                        "posterior mean: {posterior_mean:.4}"
                    )));
                }
                let mut sorted = pmf;
                sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                const SHOWN: usize = 8;
                let total_states = sorted.len();
                let shown = sorted.len().min(SHOWN);
                let remaining_mass: f64 = sorted[shown..].iter().map(|(_, p, _)| p).sum();

                for (val, prob, _lw) in sorted.into_iter().take(SHOWN) {
                    lines.push(Line::from(format!("  P({val}) = {}", fmt_prob(prob))));
                }
                if total_states > shown {
                    lines.push(Line::from(format!(
                        "  ... and {} more distinct value(s) (P = {} combined)",
                        total_states - shown,
                        fmt_prob(remaining_mass)
                    )));
                }
                lines
            }
        }
    }
}

// Mirrors runner.rs's CLI table formatting: below this threshold, fixed
// 4-decimal notation would just print "0.0000", so switch to scientific
// notation to keep small probabilities distinguishable from zero.
fn fmt_prob(prob: f64) -> String {
    if prob < 0.0001 && prob > 0.0 {
        format!("{:.4e}", prob)
    } else {
        format!("{:.4}", prob)
    }
}

fn current_log_w(current: &EnumCurrent) -> f64 {
    match current {
        EnumCurrent::Choice { machine, .. } => machine.log_w,
        EnumCurrent::Leaf { log_w, .. } => *log_w,
        EnumCurrent::Finished => 0.0,
    }
}
