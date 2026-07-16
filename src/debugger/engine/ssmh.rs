/*

Single-Site Metropolis-Hastings debug engine. SSMH doesn't pause inside a
single execution -- every iteration re-runs the whole program from
scratch, proposing a new value at one randomly chosen address while
holding every other address fixed to its current trace value, then
accepts/rejects via the Metropolis-Hastings ratio. So a "step" here is
one full MH iteration, ported directly from
`inference::ssmh::single_site_mh`'s loop body (reusing its `run_trace`
and `mh_log_alpha` helpers instead of re-deriving the accept/reject math).
There's no natural "Done" for a Markov chain, so `Continue` runs until a
configured step budget (mirrors the CLI's SSMH_STEPS + SSMH_WARMUP) or a
breakpoint on the proposed address, whichever comes first.

*/

use std::collections::HashMap;

use rand::prelude::*;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::debugger::engine::{posterior_summary_lines, StepReport};
use crate::inference::defaults::{SSMH_STEPS, SSMH_WARMUP};
use crate::inference::ssmh::{mh_log_alpha, run_trace, Trace};
use crate::interpreter::{initial_machine, Addr, Machine};
use crate::parser::value::RVal;
use crate::stats::mcmc_mean_std_err_ess;

pub struct SsmhIterationSummary {
    pub addr: Addr,
    pub curr_value: RVal,
    pub prop_value: RVal,
    pub log_alpha: f64,
    pub accepted: bool,
}

pub struct SsmhEngine {
    base_m: Machine,
    curr_val: RVal,
    curr_trace: Trace,
    // Post-warmup values only, mirrors `single_site_mh`'s `chain` -- this is
    // what the "Done" panel reports a posterior mean/CI/ESS from.
    chain: Vec<RVal>,
    iteration: usize,
    total_steps: usize,
    accepted: usize,
    attempted: usize,
    last_proposal: Option<SsmhIterationSummary>,
}

impl SsmhEngine {
    pub fn new<R: Rng + ?Sized>(program: &str, rng: &mut R) -> Result<Self, String> {
        let base_m = initial_machine(program)?;
        let (curr_val, curr_trace) = run_trace(base_m.fork(), rng, None, &HashMap::new())?;

        Ok(SsmhEngine {
            base_m,
            curr_val,
            curr_trace,
            chain: Vec::with_capacity(SSMH_STEPS),
            iteration: 0,
            total_steps: SSMH_STEPS + SSMH_WARMUP,
            accepted: 0,
            attempted: 0,
            last_proposal: None,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.iteration >= self.total_steps
    }

    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        self.last_proposal.as_ref().map(|p| &p.addr)
    }

    pub fn step<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<StepReport, String> {
        self.iteration += 1;

        let mut addresses: Vec<Addr> = self.curr_trace.values.keys().cloned().collect();
        addresses.sort();

        if addresses.is_empty() {
            if self.iteration > SSMH_WARMUP {
                self.chain.push(self.curr_val.clone());
            }
            return Ok(StepReport {
                addr_label: String::new(),
                kind: "iteration",
                detail: "no sample sites to propose on".into(),
                metric_label: "accept rate",
                metric_value: self.acceptance_rate(),
            });
        }

        self.attempted += 1;

        let a0_idx = rng.random_range(0..addresses.len());
        let a0 = addresses[a0_idx].clone();

        let (prop_val, prop_trace) =
            run_trace(self.base_m.fork(), rng, Some(&a0), &self.curr_trace.values)?;

        let log_alpha = mh_log_alpha(&self.curr_trace, &prop_trace, &a0);
        let u: f64 = rng.random();
        let accept = log_alpha >= 0.0 || u.ln() < log_alpha;

        let curr_value_before = self.curr_trace.values.get(&a0).cloned().unwrap_or(RVal::Nil);

        if accept {
            self.accepted += 1;
            self.curr_val = prop_val.clone();
            self.curr_trace = prop_trace;
        }

        self.last_proposal = Some(SsmhIterationSummary {
            addr: a0.clone(),
            curr_value: curr_value_before.clone(),
            prop_value: prop_val.clone(),
            log_alpha,
            accepted: accept,
        });

        if self.iteration > SSMH_WARMUP {
            self.chain.push(self.curr_val.clone());
        }

        Ok(StepReport {
            addr_label: a0.join("/"),
            kind: "iteration",
            detail: format!(
                "propose {curr_value_before} -> {prop_val} (log_alpha {log_alpha:.4}) -> {}",
                if accept { "ACCEPTED" } else { "rejected" }
            ),
            metric_label: "accept rate",
            metric_value: self.acceptance_rate(),
        })
    }

    fn acceptance_rate(&self) -> f64 {
        if self.attempted > 0 {
            self.accepted as f64 / self.attempted as f64
        } else {
            0.0
        }
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        if self.is_finished() {
            let mut lines = vec![
                Line::from(Span::styled(
                    "DONE (step budget reached)",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("iterations: {}/{}", self.iteration, self.total_steps)),
                Line::from(format!(
                    "current value: {}   acceptance rate: {:.1}%",
                    self.curr_val,
                    100.0 * self.acceptance_rate()
                )),
            ];
            let mut ess = None;
            lines.extend(posterior_summary_lines(&self.chain, "Estimated value", |c| {
                let (mean, std_err, chain_ess) = mcmc_mean_std_err_ess(c);
                ess = Some(chain_ess);
                (mean, std_err)
            }));
            if let Some(ess) = ess {
                lines.push(Line::from(format!(
                    "ESS (autocorrelation-adjusted): {ess:.1} / {}",
                    self.chain.len()
                )));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("[q] quit"));
            return lines;
        }

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "MCMC ITERATION ",
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}/{}", self.iteration, self.total_steps)),
            ]),
            Line::from(format!(
                "current value: {}   acceptance rate: {:.1}%",
                self.curr_val,
                100.0 * self.acceptance_rate()
            )),
        ];

        if let Some(p) = &self.last_proposal {
            let verdict_style = if p.accepted {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(format!("proposed at: {}", p.addr.join("/"))));
            lines.push(Line::from(format!(
                "{} -> {} (log_alpha {:.4})",
                p.curr_value, p.prop_value, p.log_alpha
            )));
            lines.push(Line::from(Span::styled(
                if p.accepted { "ACCEPTED" } else { "rejected" },
                verdict_style,
            )));
        } else {
            lines.push(Line::from("no proposal yet"));
        }

        lines.push(Line::from(
            "[s] propose next iteration   [c] continue   [b] toggle breakpoint on last address",
        ));
        lines
    }
}
