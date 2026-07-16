/*

Sequential Monte Carlo debug engine. SMC advances N particles in lockstep,
so there's no single trace to pause -- the natural step granularity is
"one full synchronization round": every particle runs until its next
`observe` (or Done), the population gets reweighted and resampled, and
that's what a single `step()` here does, reusing
`inference::smc::{advance_until_sync, sample_categorical, check_scm_safety}`
so the resampling math matches `run_smc` exactly instead of drifting from
it. Breakpoints key off the shared `observe` address every particle just
synchronized on.

*/

use rand::Rng;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::debugger::engine::{posterior_summary_lines, StepReport};
use crate::inference::defaults::N_PARTICLES_SMC;
use crate::inference::smc::{advance_until_sync, check_scm_safety, sample_categorical};
use crate::interpreter::{initial_machine, send, Addr, Machine, Msg};
use crate::parser::sexpr::parse;
use crate::parser::value::RVal;
use crate::stats::{effective_sample_size, sample_mean_std_err};

pub struct SmcRoundSummary {
    pub sync_addr: Addr,
    pub n_particles: usize,
    pub min_log_w: f64,
    pub max_log_w: f64,
    pub mean_log_w: f64,
    pub ess: f64,
    pub ess_pct: f64,
}

pub struct SmcEngine {
    particles: Vec<Machine>,
    n_particles: usize,
    round: usize,
    last_summary: Option<SmcRoundSummary>,
    finished: Option<Vec<RVal>>,
}

impl SmcEngine {
    pub fn new(program: &str) -> Result<Self, String> {
        // Same static safety pre-check `run_smc` does, before building anything.
        let forms = parse(program)?;
        check_scm_safety(&forms)?;

        let base_m = initial_machine(program)?;
        let n_particles = N_PARTICLES_SMC;
        let particles = (0..n_particles).map(|_| base_m.fork()).collect();

        Ok(SmcEngine {
            particles,
            n_particles,
            round: 0,
            last_summary: None,
            finished: None,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        self.last_summary.as_ref().map(|s| &s.sync_addr)
    }

    pub fn step<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<StepReport, String> {
        self.round += 1;

        let particles = std::mem::take(&mut self.particles);
        let mut log_w_starts = Vec::with_capacity(self.n_particles);
        let mut messages = Vec::with_capacity(self.n_particles);

        for p in particles.into_iter() {
            log_w_starts.push(p.log_w);
            messages.push(advance_until_sync(p, rng)?);
        }

        // All particles reached Done: settle any trailing factor()-only
        // weight (mirrors run_smc's final resampling pass) and finish.
        if messages.iter().all(|msg| matches!(msg, Msg::Done(_, _))) {
            let mut log_increments = Vec::with_capacity(self.n_particles);
            let mut finished_vals = Vec::with_capacity(self.n_particles);

            for (msg, log_w_start) in messages.into_iter().zip(log_w_starts.into_iter()) {
                if let Msg::Done(val, m) = msg {
                    log_increments.push(m.log_w - log_w_start);
                    finished_vals.push(val);
                }
            }

            let final_vals = if log_increments.iter().all(|&w| w == 0.0) {
                finished_vals
            } else {
                let max_lp = log_increments.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let weights: Vec<f64> = log_increments.iter().map(|&w| (w - max_lp).exp()).collect();
                let sum_w: f64 = weights.iter().sum();
                let probs: Vec<f64> = weights.iter().map(|w| w / sum_w).collect();

                let mut resampled = Vec::with_capacity(self.n_particles);
                for _ in 0..self.n_particles {
                    let parent_idx = sample_categorical(&probs, rng);
                    resampled.push(finished_vals[parent_idx].clone());
                }
                resampled
            };

            self.finished = Some(final_vals);

            return Ok(StepReport {
                addr_label: String::new(),
                kind: "done",
                detail: format!("all {} particles finished", self.n_particles),
                metric_label: "log_w",
                metric_value: 0.0,
            });
        }

        let mut log_increments = Vec::with_capacity(self.n_particles);
        let mut paused_machines = Vec::with_capacity(self.n_particles);
        let mut sync_addr: Option<Addr> = None;

        for (msg, log_w_start) in messages.into_iter().zip(log_w_starts.into_iter()) {
            match msg {
                Msg::Observe(addr, dist, y_obs, mut m) => {
                    if sync_addr.is_none() {
                        sync_addr = Some(addr);
                    }
                    let lp = dist.log_prob(&y_obs);
                    m.log_w += lp;
                    log_increments.push(m.log_w - log_w_start);
                    send(&mut m, y_obs);
                    paused_machines.push(m);
                }
                _ => {
                    return Err(
                        "SMC Desynchronization Error: particles reached divergent execution states. \
                         All particles in Sequential Monte Carlo must encounter the exact same sequence \
                         of 'observe' statements."
                            .into(),
                    );
                }
            }
        }

        let max_lp = log_increments.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let weights: Vec<f64> = log_increments.iter().map(|&w| (w - max_lp).exp()).collect();
        let sum_w: f64 = weights.iter().sum();
        let probs: Vec<f64> = weights.iter().map(|w| w / sum_w).collect();

        let mut new_particles = Vec::with_capacity(self.n_particles);
        for _ in 0..self.n_particles {
            let parent_idx = sample_categorical(&probs, rng);
            new_particles.push(paused_machines[parent_idx].fork());
        }

        let ess = effective_sample_size(&probs);
        let ess_pct = 100.0 * ess / self.n_particles as f64;
        let min_log_w = log_increments.iter().cloned().fold(f64::INFINITY, f64::min);
        let mean_log_w = log_increments.iter().sum::<f64>() / log_increments.len() as f64;
        let sync_addr = sync_addr.expect("at least one particle synchronized on an observe");

        self.particles = new_particles;
        self.last_summary = Some(SmcRoundSummary {
            sync_addr: sync_addr.clone(),
            n_particles: self.n_particles,
            min_log_w,
            max_log_w: max_lp,
            mean_log_w,
            ess,
            ess_pct,
        });

        Ok(StepReport {
            addr_label: sync_addr.join("/"),
            kind: "round",
            detail: format!("resampled {} particles (ESS {ess_pct:.1}%)", self.n_particles),
            metric_label: "mean log_w",
            metric_value: mean_log_w,
        })
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        if let Some(finished) = &self.finished {
            let preview: Vec<String> = finished.iter().take(5).map(|v| v.to_string()).collect();
            let mut lines = vec![
                Line::from(Span::styled(
                    "DONE",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("{} particles finished", finished.len())),
            ];
            lines.extend(posterior_summary_lines(
                finished,
                "Estimated expected value",
                sample_mean_std_err,
            ));
            lines.push(Line::from(format!(
                "sample of results: {}",
                preview.join(", ")
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("[q] quit"));
            return lines;
        }

        match &self.last_summary {
            None => vec![
                Line::from(Span::styled(
                    "READY",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!(
                    "{} particles initialized, no sync round run yet",
                    self.n_particles
                )),
                Line::from(""),
                Line::from("[s] run next synchronization round   [c] continue"),
            ],
            Some(s) => vec![
                Line::from(vec![
                    Span::styled(
                        "SYNC ROUND ",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!("{} @ {}", self.round, s.sync_addr.join("/"))),
                ]),
                Line::from(format!(
                    "log_w range: [{:.4}, {:.4}]   mean: {:.4}",
                    s.min_log_w, s.max_log_w, s.mean_log_w
                )),
                Line::from(format!("ESS: {:.1} / {} ({:.1}%)", s.ess, s.n_particles, s.ess_pct)),
                Line::from(""),
                Line::from("[s] next round   [c] continue   [b] toggle breakpoint at this observe"),
            ],
        }
    }
}
