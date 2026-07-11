/*
Module that implements the Likelihood Weighting inference algorithm, the most
elementary of the inference algorithms.
It runs the program straight through without ever pausing to switch trajectories.
*/

use crate::interpreter::{initial_machine, resume, send, Machine, Msg};
use crate::parser::value::RVal;
use rand::prelude::*;


pub fn run_lw<R: Rng + ?Sized>(mut m: Machine, rng: &mut R) -> Result<(RVal, f64), String> {
    loop {
        // We pass the machine by value without needing to use .clone()
        match resume(m)? {
            Msg::Done(val, next_m) => {
                return Ok((val, next_m.log_w));
            }

            Msg::Sample(_addr, dist, mut next_m) => {
                // 1. Sample from the prior distribution
                let sample_val = dist.sample(rng);

                // 2. Inject the value and continue
                send(&mut next_m, sample_val);

                m = next_m;
            }

            Msg::Observe(_addr, dist, y_obs, mut next_m) => {
                // 1. Accumulate the log-likelihood and continue
                next_m.log_w += dist.log_prob(&y_obs);

                // 2. Inject the observed value so the program keeps going
                send(&mut next_m, y_obs);

                m = next_m;
            }

            Msg::Factor(_addr, _w ,  next_m ) => {
                m = next_m;
            }
        }
    }
}

pub fn likelihood_weighting<R: Rng + ?Sized>(
    program: &str,
    n_particles: usize,
    rng: &mut R,
) -> Result<(Vec<RVal>, Vec<f64>), String> {
    let mut values = Vec::with_capacity(n_particles);
    let mut log_weights = Vec::with_capacity(n_particles);
    let base_m = initial_machine(program)?;

    // Run the program N times, cloning (forking) the base machine's memory
    for _ in 0..n_particles {
        let (val, log_w) = run_lw(base_m.fork(), rng)?;
        values.push(val);
        log_weights.push(log_w);
    }

    // 1. Find the maximum log_weight to avoid overflow/underflow
    let max_lw = log_weights
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // 2. Exponentiate after subtracting the max: exp(w_i - max_w)
    let exp_weights: Vec<f64> = log_weights
        .iter()
        .map(|&w| (w - max_lw).exp())
        .collect();

    // 3. Sum all the weights
    let sum_exp: f64 = exp_weights.iter().sum();

    let normalized_weights: Vec<f64> = exp_weights
        .iter()
        .map(|w| w / sum_exp)
        .collect();

    // Return the results
    Ok((values, normalized_weights))
}