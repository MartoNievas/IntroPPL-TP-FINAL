/*

Module that implements the Sequential Monte Carlo (SMC) inference algorithm, also
known as a particle filter. Instead of running one trace at a time, SMC advances a
whole population of N particles (Vec<Machine>) in lockstep: each particle runs until
it hits the next 'observe', the population is reweighted by the resulting likelihoods,
and particles are then resampled proportionally to their weight before continuing.
This is where the `Machine` abstraction really shines, since forking a particle's
state is just a cheap memory clone.

Because every particle must reach the same sequence of 'observe' statements at the
same time, this module also performs a static safety check on the program's AST
before running: it rejects models where an 'observe' could occur in a
non-deterministic position (e.g. inside an 'if' whose condition depends on a random
variable, or inside a function body), since that would desynchronize the particle
population at runtime.

The check is a small taint analysis: every expression is classified as `Det`
(its value is the same in all particles) or `Rand` (it may differ between particles).
An 'if' with an 'observe' in one of its branches is only rejected when its condition
is `Rand`; with a `Det` condition all particles take the same branch and stay in sync.

*/

use crate::interpreter::{Machine, Msg, initial_machine, resume, send};
use crate::parser::sexpr::Form;
use crate::parser::sexpr::parse;
use crate::parser::value::RVal;
use rand::prelude::*;
use std::collections::HashMap;

/// Runs the Sequential Monte Carlo algorithm with N particles.
pub fn run_smc<R: Rng + ?Sized>(
    program: &str,
    n_particles: usize,
    rng: &mut R,
) -> Result<Vec<RVal>, String> {
    // Perform the static checks on the AST here
    let forms = parse(program)?;

    // Check the forms
    check_scm_safety(&forms)?;

    // Parse the AST once and initialize it on the base machine
    let base_m = initial_machine(program)?;

    // 1. Initialize the N particles using ultra-fast memory cloning
    let mut particles: Vec<Machine> = Vec::with_capacity(n_particles);
    for _ in 0..n_particles {
        particles.push(base_m.fork());
    }

    loop {
        // 2. Advance all particles until their next synchronization point.
        // We record each particle's log_w *before* advancing, so we can
        // later recover exactly how much weight it picked up between syncs
        // (factor calls included, not just the observe at the sync point).
        let mut log_w_starts = Vec::with_capacity(n_particles);
        let mut messages = Vec::with_capacity(n_particles);

        for p in particles.into_iter() {
            log_w_starts.push(p.log_w);
            messages.push(advance_until_sync(p, rng)?);
        }

        // 3. If all particles finished the program, they still may have
        // picked up weight from a `factor` call after the last `observe`
        // (or, for factor-only models, throughout the entire run) that was
        // never accounted for, since `factor` never pauses the machine and
        // therefore never triggers the reweighting step below. We recover
        // that contribution here with one last importance-weighted
        // resampling pass over the log_w delta since the last sync point.
        if messages.iter().all(|msg| matches!(msg, Msg::Done(_, _))) {
            let mut log_increments = Vec::with_capacity(n_particles);
            let mut finished = Vec::with_capacity(n_particles);

            for (msg, log_w_start) in messages.into_iter().zip(log_w_starts.into_iter()) {
                if let Msg::Done(val, m) = msg {
                    log_increments.push(m.log_w - log_w_start);
                    finished.push(val);
                }
            }

            // If nothing changed since the last sync point (the common
            // case: a model that ends right after an observe, with no
            // trailing factor), skip the extra resampling pass entirely --
            // it would only add unnecessary variance to models that were
            // already correct.
            if log_increments.iter().all(|&w| w == 0.0) {
                return Ok(finished);
            }

            let max_lp = log_increments
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let weights: Vec<f64> = log_increments.iter().map(|&w| (w - max_lp).exp()).collect();
            let sum_w: f64 = weights.iter().sum();
            let probs: Vec<f64> = weights.iter().map(|w| w / sum_w).collect();

            let mut resampled = Vec::with_capacity(n_particles);
            for _ in 0..n_particles {
                let parent_idx = sample_categorical(&probs, rng);
                resampled.push(finished[parent_idx].clone());
            }
            return Ok(resampled);
        }

        // 4. Process the observation step
        let mut log_increments = Vec::with_capacity(n_particles);
        let mut paused_machines = Vec::with_capacity(n_particles);

        for (msg, log_w_start) in messages.into_iter().zip(log_w_starts.into_iter()) {
            match msg {
                Msg::Observe(_addr, dist, y_obs, mut m) => {

                    let lp = dist.log_prob(&y_obs);

                    m.log_w += lp;

                    // Real delta since the last synchronization point:
                    // includes this observe PLUS any factor() the particle
                    // went through along the way. If we only used `lp`, a
                    // factor between two observes (or before the first one)
                    // would be left out of the resampling step.
                    let increment = m.log_w - log_w_start;
                    log_increments.push(increment);

                    // Inject the observed value so the machine can continue
                    send(&mut m, y_obs);
                    paused_machines.push(m);
                }
                // Dynamic runtime detection of desynchronization
                _ => return Err("SMC Desynchronization Error: Particles reached divergent execution states. All particles in Sequential Monte Carlo must encounter the exact same sequence of 'observe' statements.".into()),
            }
        }

        // 5. Numerically stable softmax normalization
        let max_lp = log_increments
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let weights: Vec<f64> = log_increments.iter().map(|&w| (w - max_lp).exp()).collect();

        let sum_w: f64 = weights.iter().sum();
        let probs: Vec<f64> = weights.iter().map(|w| w / sum_w).collect();

        // 6. Multinomial resampling
        let mut new_particles = Vec::with_capacity(n_particles);
        for _ in 0..n_particles {
            let parent_idx = sample_categorical(&probs, rng);
            // Here it's legitimate and necessary to use .fork() to duplicate the winners
            new_particles.push(paused_machines[parent_idx].fork());
        }
        particles = new_particles;
    }
}

// ---------------------------------------------------------------------------
// Static AST analysis (taint analysis) for SMC synchronization
// ---------------------------------------------------------------------------

/// Two-point lattice: `Det < Rand`, so `max` is the join.
/// `Det`  -> the value is identical in every particle.
/// `Rand` -> the value may differ between particles (depends on a `sample`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Taint {
    Det,
    Rand,
}

/// Result of analyzing one expression.
struct Info {
    taint: Taint,
    has_observe: bool,
}

impl Info {
    fn pure(taint: Taint) -> Self {
        Info {
            taint,
            has_observe: false,
        }
    }
}

/// Maps each locally bound variable (let / fn parameter) to its taint.
/// A symbol that is not in the environment is a global primitive: `Det`.
type TaintEnv = HashMap<String, Taint>;

// Helper function for the static AST analysis that detects desynchronization in the SMC algorithm
pub(crate) fn check_scm_safety(forms: &[Form]) -> Result<(), String> {
    let env = TaintEnv::new();
    for form in forms {
        check_form(form, &env)?;
    }
    Ok(())
}

// Analyzes every form in `items` and joins the results:
// taint = max of the taints, has_observe = OR of the flags.
fn check_args(items: &[Form], env: &TaintEnv) -> Result<Info, String> {
    let mut acc = Info::pure(Taint::Det);
    for item in items {
        let info = check_form(item, env)?;
        acc.taint = acc.taint.max(info.taint);
        acc.has_observe |= info.has_observe;
    }
    Ok(acc)
}

// Recursive function that computes, for a form, its taint (Det/Rand) and whether it
// contains at least one `observe`. Fails if it finds an `observe` in a structurally
// unsafe position:
//   - inside an `if` branch whose condition is Rand
//   - inside a `fn` body
//
// NOTE on `factor`: `factor` never pauses the machine (see FactorK in runtime.rs),
// so it does not need the same synchronization guarantees as `observe`: particles
// don't need to reach it in the same order for resampling to stay valid, because
// there is no Msg to intercept at that point. Its arguments are still walked in
// case they contain a nested `observe`, and its result (nil) is always Det.
//
// Calls: a head symbol that is bound in the environment (let variable or fn
// parameter) is a user-defined / unknown function, so its result is conservatively
// Rand. A head symbol that is not bound is a global primitive, so the result is
// the join of its arguments' taints.
fn check_form(form: &Form, env: &TaintEnv) -> Result<Info, String> {
    match form {
        Form::Int(_) | Form::Float(_) | Form::Bool(_) | Form::Str(_) | Form::Nil => {
            Ok(Info::pure(Taint::Det))
        }

        Form::Symbol(name) => Ok(Info::pure(env.get(name).copied().unwrap_or(Taint::Det))),

        Form::List(list, _list_type) => {
            if list.is_empty() {
                return Ok(Info::pure(Taint::Det));
            }

            if let Form::Symbol(head) = &list[0] {
                match head.as_str() {
                    "sample" => {
                        // Check the arguments in case they contain nested observes
                        let mut info = check_args(&list[1..], env)?;
                        info.taint = Taint::Rand;
                        Ok(info)
                    }

                    "observe" => {
                        // Check the arguments in case they contain nested observes
                        let mut info = check_args(&list[1..], env)?;
                        info.has_observe = true; // Report upward that we found an observe
                        Ok(info)
                    }

                    "factor" => {
                        let mut info = check_args(&list[1..], env)?;
                        info.taint = Taint::Det; // factor always returns nil
                        Ok(info)
                    }

                    "if" => {
                        if list.len() == 4 {
                            let cond = check_form(&list[1], env)?;
                            let then_info = check_form(&list[2], env)?;
                            let else_info = check_form(&list[3], env)?;

                            // Only a random condition can send particles down different
                            // branches. With a deterministic condition they all take the
                            // same one and stay synchronized.
                            if cond.taint == Taint::Rand
                                && (then_info.has_observe || else_info.has_observe)
                            {
                                return Err(
                                    "SMC Static Analysis Error: Found an 'observe' statement inside an 'if' branch whose condition depends on a random variable. \
                                     Particles may take different branches and desynchronize. Please move the observation outside the conditional \
                                     or make the condition deterministic.".into()
                                );
                            }

                            return Ok(Info {
                                // If the condition is Rand, the result is Rand even when
                                // both branches are Det: which one was chosen is random.
                                taint: cond.taint.max(then_info.taint).max(else_info.taint),
                                has_observe: cond.has_observe
                                    || then_info.has_observe
                                    || else_info.has_observe,
                            });
                        }
                        // Malformed `if`: walk the children conservatively
                        check_args(&list[1..], env)
                    }

                    "fn" | "defn" => {
                        let start_idx = if head.as_str() == "defn" { 3 } else { 2 };

                        // Parameters may receive random arguments: treat them as Rand
                        let mut local = env.clone();
                        if let Some(Form::List(params, _)) = list.get(start_idx - 1) {
                            for p in params {
                                if let Form::Symbol(n) = p {
                                    local.insert(n.clone(), Taint::Rand);
                                }
                            }
                        }

                        for expr in list.iter().skip(start_idx) {
                            let info = check_form(expr, &local)?;

                            if info.has_observe {
                                return Err(
                                    "SMC Static Analysis Error: Found an 'observe' statement inside a 'fn' definition. \
                                     Functions can be called dynamically, which breaks SMC synchronization guarantees.".into()
                                );
                            }
                        }
                        // The closure value itself is the same in every particle
                        Ok(Info::pure(Taint::Det))
                    }

                    "let" => {
                        if list.len() >= 3 {
                            if let Form::List(binds, _list_type) = &list[1] {
                                let mut local = env.clone();
                                let mut has_obs = false;

                                // Bindings are sequential: each value sees the previous ones
                                for pair in binds.chunks(2) {
                                    if pair.len() == 2 {
                                        let info = check_form(&pair[1], &local)?;
                                        has_obs |= info.has_observe;
                                        if let Form::Symbol(name) = &pair[0] {
                                            local.insert(name.clone(), info.taint);
                                        }
                                    }
                                }

                                // The taint of the let is the taint of its last body expression
                                let mut taint = Taint::Det;
                                for expr in &list[2..] {
                                    let info = check_form(expr, &local)?;
                                    has_obs |= info.has_observe;
                                    taint = info.taint;
                                }
                                return Ok(Info {
                                    taint,
                                    has_observe: has_obs,
                                });
                            }
                        }
                        check_args(&list[1..], env)
                    }

                    _ => {
                        // Standard call. Check its arguments.
                        let mut info = check_args(&list[1..], env)?;
                        if env.contains_key(head.as_str()) {
                            // Call to a user-defined / unknown function: assume it
                            // may be random.
                            info.taint = Taint::Rand;
                        }
                        Ok(info)
                    }
                }
            } else {
                // If the first element is not a symbol (e.g. ((self self) p)), the
                // function is computed dynamically: assume the result is Rand.
                let mut info = check_args(list, env)?;
                info.taint = Taint::Rand;
                Ok(info)
            }
        }
    }
}

// Helper function to advance until the next 'Observe' or until the program finishes.
// Intermediate samples are resolved automatically by sampling from the prior.
pub(crate) fn advance_until_sync<R: Rng + ?Sized>(
    mut m: Machine,
    rng: &mut R,
) -> Result<Msg, String> {
    loop {
        match resume(m)? {
            Msg::Sample(_addr, dist, mut next_m) => {
                // Sample from the prior distribution
                let sample_val = dist.sample(rng);
                send(&mut next_m, sample_val);
                m = next_m;
            }
            // New case
            Msg::Factor(_addr, w, mut next_m) => {
                next_m.log_w += w;
                send(&mut next_m, RVal::Nil);
                m = next_m;
            }
            // Once we hit an Observe or Done, return the message to the controller
            other => return Ok(other),
        }
    }
}

// Helper function for resampling: selects an index according to its categorical probabilities.
pub(crate) fn sample_categorical<R: Rng + ?Sized>(probs: &[f64], rng: &mut R) -> usize {
    let u: f64 = rng.random();
    let mut cumsum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        cumsum += p;
        if u <= cumsum {
            return i;
        }
    }
    probs.len() - 1
}

