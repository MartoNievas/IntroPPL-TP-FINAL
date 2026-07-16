/*

Dispatches the terminal debugger's step/continue/breakpoint/render logic
across the five inference algorithms. Each algorithm's execution model is
different enough (a single linear trace for LW, a branching tree for
Exact Enumeration, a synchronized particle population for SMC, a
re-run-and-propose Markov chain for SSMH, a gradient optimization loop
for BBVI) that a single `PausedAt`-style enum can't represent all of
them. Instead, each algorithm gets its own self-contained `*Engine`
struct in its own module, and `Engine` is a closed enum that dispatches
to whichever one is active -- following the same "enum + exhaustive
match over a trait object" preference the rest of this project already
uses (see the CEK-vs-CPS discussion in the README) rather than
`Box<dyn Trait>`.

*/

pub mod bbvi;
pub mod enumeration;
pub mod lw;
pub mod smc;
pub mod ssmh;

use std::collections::HashMap;

use rand::rngs::StdRng;
use ratatui::text::Line;

use crate::cli::Algorithm;
use crate::interpreter::Addr;
use crate::parser::value::RVal;
use crate::stats::{ci95_margin, is_numeric};

/// What a single `step()` did, in a shape generic enough for the shared
/// event-log panel regardless of which algorithm produced it. `metric_label`
/// names whatever headline number the algorithm tracks (`log_w` for the
/// probabilistic trace/branch/particle algorithms, `ELBO` for BBVI).
pub struct StepReport {
    pub addr_label: String,
    pub kind: &'static str,
    pub detail: String,
    pub metric_label: &'static str,
    pub metric_value: f64,
}

pub enum Engine {
    Lw(lw::LwEngine),
    Enumeration(enumeration::EnumEngine),
    Smc(smc::SmcEngine),
    Ssmh(ssmh::SsmhEngine),
    Bbvi(bbvi::BbviEngine),
}

impl Engine {
    pub fn new(program: &str, algorithm: Algorithm, rng: &mut StdRng) -> Result<Self, String> {
        Ok(match algorithm {
            Algorithm::Lw => Engine::Lw(lw::LwEngine::new(program)?),
            Algorithm::Enumeration => Engine::Enumeration(enumeration::EnumEngine::new(program)?),
            Algorithm::Smc => Engine::Smc(smc::SmcEngine::new(program)?),
            Algorithm::Ssmh => Engine::Ssmh(ssmh::SsmhEngine::new(program, rng)?),
            Algorithm::Bbvi => Engine::Bbvi(bbvi::BbviEngine::new(program)?),
        })
    }

    pub fn is_finished(&self) -> bool {
        match self {
            Engine::Lw(e) => e.is_finished(),
            Engine::Enumeration(e) => e.is_finished(),
            Engine::Smc(e) => e.is_finished(),
            Engine::Ssmh(e) => e.is_finished(),
            Engine::Bbvi(e) => e.is_finished(),
        }
    }

    /// Advances the active engine by exactly one of its own units of work
    /// (one probabilistic effect for LW, one branch descent for
    /// Enumeration, one particle-population sync round for SMC, one MH
    /// iteration for SSMH, one Adam step for BBVI). No-op if already
    /// finished -- callers should check `is_finished()` first to avoid
    /// polluting history with empty steps.
    pub fn step(&mut self, rng: &mut StdRng) -> Result<StepReport, String> {
        match self {
            Engine::Lw(e) => e.step(rng),
            Engine::Enumeration(e) => e.step(),
            Engine::Smc(e) => e.step(rng),
            Engine::Ssmh(e) => e.step(rng),
            Engine::Bbvi(e) => e.step(rng),
        }
    }

    /// Only meaningful for Exact Enumeration (moves the highlighted branch
    /// in the finite-support picker); a no-op for every other engine.
    pub fn select_prev(&mut self) {
        if let Engine::Enumeration(e) = self {
            e.select_prev();
        }
    }

    pub fn select_next(&mut self) {
        if let Engine::Enumeration(e) = self {
            e.select_next();
        }
    }

    /// The address a breakpoint would key off of if toggled/checked right
    /// now, or `None` if the engine has no such notion at this point (e.g.
    /// BBVI never does, LW/Enumeration don't once they reach Done).
    pub fn current_breakpoint_addr(&self) -> Option<&Addr> {
        match self {
            Engine::Lw(e) => e.current_breakpoint_addr(),
            Engine::Enumeration(e) => e.current_breakpoint_addr(),
            Engine::Smc(e) => e.current_breakpoint_addr(),
            Engine::Ssmh(e) => e.current_breakpoint_addr(),
            Engine::Bbvi(e) => e.current_breakpoint_addr(),
        }
    }

    pub fn render_current(&self) -> Vec<Line<'static>> {
        match self {
            Engine::Lw(e) => e.render_current(),
            Engine::Enumeration(e) => e.render_current(),
            Engine::Smc(e) => e.render_current(),
            Engine::Ssmh(e) => e.render_current(),
            Engine::Bbvi(e) => e.render_current(),
        }
    }

    /// Extra line appended to the bottom help bar, describing any
    /// algorithm-specific keybinding behavior (e.g. branch selection for
    /// Enumeration, breakpoints not applying to BBVI). Empty when the
    /// generic footer already covers everything.
    pub fn help_hint(&self) -> &'static str {
        match self {
            Engine::Lw(_) => "",
            Engine::Enumeration(e) => e.help_hint(),
            Engine::Smc(_) => "",
            Engine::Ssmh(_) => "",
            Engine::Bbvi(_) => "[b] breakpoints don't apply to BBVI (no addresses to pause on)",
        }
    }
}

/// Final-result summary shared by the SMC/SSMH/BBVI "Done" panels, mirroring
/// what `runner::run_algorithm_on_model` prints at the end of a non-debug
/// run: a posterior mean +/- standard error and a 95% CI when the result
/// population is numeric, or a frequency breakdown otherwise. `stats` is only
/// invoked once `vals` is confirmed numeric, so each caller can plug in
/// whichever standard-error estimator matches its algorithm (plain sample
/// std err for SMC/BBVI, autocorrelation-adjusted for SSMH) without risking
/// a panic on a non-numeric result.
pub fn posterior_summary_lines(
    vals: &[RVal],
    label: &str,
    stats: impl FnOnce(&[RVal]) -> (f64, f64),
) -> Vec<Line<'static>> {
    if vals.is_empty() || !vals.iter().all(is_numeric) {
        return categorical_summary_lines(vals);
    }

    let (mean, std_err) = stats(vals);
    let margin = ci95_margin(std_err);
    vec![
        Line::from(format!("{label}: {mean:.4} \u{b1} {std_err:.4}")),
        Line::from(format!(
            "95% CI: [{:.4}, {:.4}]",
            mean - margin,
            mean + margin
        )),
    ]
}

fn categorical_summary_lines(vals: &[RVal]) -> Vec<Line<'static>> {
    if vals.is_empty() {
        return vec![Line::from("(no results)")];
    }

    let n = vals.len() as f64;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for v in vals {
        *counts.entry(v.to_string()).or_insert(0) += 1;
    }
    let mut entries: Vec<(String, usize)> = counts.into_iter().collect();
    entries.sort_by_key(|&(_, c)| std::cmp::Reverse(c));

    let mut lines = vec![Line::from(
        "Non-numeric result -- posterior distribution (frequency):",
    )];
    for (val, c) in entries.into_iter().take(5) {
        lines.push(Line::from(format!(
            "  {val}: {:.4} ({c}/{n})",
            c as f64 / n
        )));
    }
    lines
}
