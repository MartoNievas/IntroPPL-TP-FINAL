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
non-deterministic position (e.g. inside an 'if' branch or a function body), since
that would desynchronize the particle population at runtime.

*/

use crate::interpreter::{initial_machine, resume, send, Machine, Msg};
use crate::parser::value::RVal;
use rand::prelude::*;
use crate::parser::sexpr::Form;
use crate::parser::sexpr::parse;

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
            let weights: Vec<f64> = log_increments
                .iter()
                .map(|&w| (w - max_lp).exp())
                .collect();
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

        let weights: Vec<f64> = log_increments
            .iter()
            .map(|&w| (w - max_lp).exp())
            .collect();

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

// Helper function for the static AST analysis that detects desynchronization in the SMC algorithm
fn check_scm_safety(forms: &[Form]) -> Result<(), String> {

    for form in forms {
        check_form(form)?;
    }
    Ok(())
}

// Recursive function that returns true if the form contains at least one `observe`.
// Fails if it finds an `observe` in a structurally unsafe position
//
// NOTE on `factor`: we deliberately do NOT treat "factor" as its own case
// here. `factor` never pauses the machine (see FactorK in runtime.rs), so it
// does not need the same synchronization guarantees as `observe`: particles
// don't need to reach it in the same order for resampling to stay valid,
// because there is no Msg to intercept at that point. It falls into the `_`
// arm (standard call) and its arguments are still walked in case they
// contain a nested `observe`.
fn check_form(form: &Form) -> Result<bool, String> {
    match form {
        Form::Int(_) | Form::Float(_) | Form::Bool(_) | Form::Str(_) | Form::Nil | Form::Symbol(_) => {
            Ok(false)
        }
        
        Form::List(list, _list_type) => {
            if list.is_empty() {
                return Ok(false);
            }

            if let Form::Symbol(head) = &list[0] {
                match head.as_str() {
                    "observe" => {
                        // Check the arguments in case they contain nested observes
                        for arg in &list[1..] {
                            check_form(arg)?;
                        }
                        Ok(true) // Report upward that we found an observe
                    }
                    
                    "if" => {
                        if list.len() == 4 {
                            let _ = check_form(&list[1])?; // Test
                            
                            let then_has_obs = check_form(&list[2])?;
                            let else_has_obs = check_form(&list[3])?;
                            
                            
                            if then_has_obs || else_has_obs {
                                return Err(
                                    "SMC Static Analysis Error: Found an 'observe' statement inside an 'if' branch. \
                                     SMC requires a deterministic observation flow. Please move the observation outside the conditional.".into()
                                );
                            }
                            return Ok(false);
                        }
                        Ok(false)
                    }
                    
                    "fn" | "defn" => {
                        let start_idx = if head.as_str() == "defn" { 3 } else { 2 };
                        
                        if list.len() > start_idx {
                            for expr in &list[start_idx..] {
                                let has_obs = check_form(expr)?;
                                
                                
                                if has_obs {
                                    return Err(
                                        "SMC Static Analysis Error: Found an 'observe' statement inside a 'fn' definition. \
                                         Functions can be called dynamically, which breaks SMC synchronization guarantees.".into()
                                    );
                                }
                            }
                        }
                        Ok(false)
                    }
                    
                    "let" => {
                        if list.len() >= 3 {
                            if let Form::List(binds, _list_type) = &list[1] {
                                let mut has_obs = false;
                                // Check the expressions assigned to the variables
                                for i in (1..binds.len()).step_by(2) {
                                    has_obs |= check_form(&binds[i])?;
                                }
                                // Check the body of the let
                                for expr in &list[2..] {
                                    has_obs |= check_form(expr)?;
                                }
                                return Ok(has_obs);
                            }
                        }
                        Ok(false)
                    }
                    
                    _ => {
                        // Standard call. Check its arguments.
                        let mut has_obs = false;
                        for arg in list {
                            has_obs |= check_form(arg)?;
                        }
                        Ok(has_obs)
                    }
                }
            } else {
                // If the first element is not a symbol, check the whole list
                let mut has_obs = false;
                for arg in list {
                    has_obs |= check_form(arg)?;
                }
                Ok(has_obs)
            }
        }
    }
}



// Helper function to advance until the next 'Observe' or until the program finishes.
// Intermediate samples are resolved automatically by sampling from the prior.
fn advance_until_sync<R: Rng + ?Sized>(mut m: Machine, rng: &mut R) -> Result<Msg, String> {
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
fn sample_categorical<R: Rng + ?Sized>(probs: &[f64], rng: &mut R) -> usize {
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