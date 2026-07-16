/*

Black-Box Variational Inference debug engine. BBVI has no addresses to
pause on at all -- it's a gradient optimization loop over variational
parameters theta, batching `n_samples` full program traces per step. So
"step" here is one full Adam iteration, ported directly from
`inference::bbvi::run_bbvi`'s loop body (batch of `run_bbvi_sample`
traces, baseline-centered score-function gradient, `AdamOptimizer::step`).
Breakpoints don't apply here (`current_breakpoint_addr` always returns
`None`); `Continue` just runs to the configured step budget.

*/

use std::collections::HashMap;

use rand::Rng;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::debugger::engine::{posterior_summary_lines, StepReport};
use crate::inference::bbvi::{run_bbvi_sample, AdamOptimizer};
use crate::inference::defaults::{BBVI_LR, BBVI_SAMPLES, BBVI_STEPS};
use crate::interpreter::{initial_machine, Addr, Machine};
use crate::parser::distribution::Distribution;
use crate::parser::value::RVal;
use crate::stats::sample_mean_std_err;

pub struct BbviEngine {
    base_m: Machine,
    guides: HashMap<Addr, Distribution>,
    theta: HashMap<Addr, Vec<f64>>,
    optimizer: AdamOptimizer,
    elbo_history: Vec<f64>,
    last_batch_vals: Vec<RVal>,
    step_idx: usize,
    total_steps: usize,
    n_samples: usize,
}

impl BbviEngine {
    pub fn new(program: &str) -> Result<Self, String> {
        let base_m = initial_machine(program)?;
        Ok(BbviEngine {
            base_m,
            guides: HashMap::new(),
            theta: HashMap::new(),
            optimizer: AdamOptimizer::new(BBVI_LR),
            elbo_history: Vec::new(),
            last_batch_vals: Vec::new(),
            step_idx: 0,
            total_steps: BBVI_STEPS,
            n_samples: BBVI_SAMPLES,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.step_idx >= self.total_steps
    }

    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        None
    }

    pub fn step<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Result<StepReport, String> {
        self.step_idx += 1;

        let mut step_elbos = Vec::with_capacity(self.n_samples);
        let mut step_scores = Vec::with_capacity(self.n_samples);
        self.last_batch_vals.clear();

        for _ in 0..self.n_samples {
            let res = run_bbvi_sample(self.base_m.fork(), &mut self.guides, &mut self.theta, rng)?;
            step_elbos.push(res.elbo_sample);
            step_scores.push(res.scores);
            self.last_batch_vals.push(res.val);
        }

        let mean_elbo: f64 = step_elbos.iter().sum::<f64>() / (self.n_samples as f64);
        let prev_elbo = self.elbo_history.last().copied();
        self.elbo_history.push(mean_elbo);

        let mut grad_accum: HashMap<Addr, Vec<f64>> = HashMap::new();
        for (elbo_i, scores_i) in step_elbos.iter().zip(step_scores.iter()) {
            let reward = elbo_i - mean_elbo;
            for (addr, grad_i) in scores_i {
                let acc = grad_accum
                    .entry(addr.clone())
                    .or_insert_with(|| vec![0.0; grad_i.len()]);
                for (k, &g_val) in grad_i.iter().enumerate() {
                    acc[k] += reward * g_val / (self.n_samples as f64);
                }
            }
        }

        self.optimizer.step(&mut self.theta, &grad_accum);

        let delta = prev_elbo.map(|p| mean_elbo - p).unwrap_or(0.0);

        Ok(StepReport {
            addr_label: String::new(),
            kind: "adam",
            detail: format!(
                "mean ELBO {mean_elbo:.4} (delta {delta:+.4}) over {} guided direction(s)",
                self.theta.len()
            ),
            metric_label: "ELBO",
            metric_value: mean_elbo,
        })
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        let last_elbo = self.elbo_history.last().copied().unwrap_or(0.0);
        let first_elbo = self.elbo_history.first().copied().unwrap_or(0.0);

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    if self.is_finished() { "DONE " } else { "OPTIMIZING " },
                    Style::default()
                        .fg(if self.is_finished() { Color::Blue } else { Color::Cyan })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("step {}/{}", self.step_idx, self.total_steps)),
            ]),
            Line::from(format!(
                "ELBO: {last_elbo:.4}   (initial {first_elbo:.4}, delta {:+.4})",
                last_elbo - first_elbo
            )),
        ];

        let mut addrs: Vec<&Addr> = self.theta.keys().collect();
        addrs.sort();
        for addr in addrs.into_iter().take(3) {
            let params = &self.theta[addr];
            let fmt_params: Vec<String> = params.iter().map(|p| format!("{p:.3}")).collect();
            lines.push(Line::from(format!(
                "  theta[{}] = [{}]",
                addr.join("/"),
                fmt_params.join(", ")
            )));
        }

        if !self.last_batch_vals.is_empty() {
            let preview: Vec<String> = self
                .last_batch_vals
                .iter()
                .take(5)
                .map(|v| v.to_string())
                .collect();
            lines.push(Line::from(format!(
                "  last batch sample of results: {}",
                preview.join(", ")
            )));
        }

        if self.is_finished() {
            lines.extend(posterior_summary_lines(
                &self.last_batch_vals,
                "Estimated posterior mean (via guide)",
                sample_mean_std_err,
            ));
        }

        lines.push(Line::from(if self.is_finished() {
            "[q] quit"
        } else {
            "[s] run next Adam step   [c] continue"
        }));

        lines
    }
}
