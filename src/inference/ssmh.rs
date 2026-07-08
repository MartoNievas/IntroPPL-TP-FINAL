/*

Modulo que implemente el algoritmo de inferencia Single Site Metropolis-Hastings / SSMH, este es un algoritmo Markov Chain Monte Carlo / MCMC
El mismo realiza una camina aleatoria sobre la traza de ejecucion.

*/

use crate::interpreter::{Addr, Machine, Msg, initial_machine, resume, send};
use crate::parser::value::RVal;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

/// La Traza captura el historial completo de una ejecución del programa.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub values: HashMap<Addr, RVal>,
    pub sample_log_probs: HashMap<Addr, f64>,
    pub observe_log_probs: HashMap<Addr, f64>,
}

pub fn single_site_mh<R: Rng + ?Sized>(
    program: &str,
    rng: &mut R,
    steps: usize,
    warmup: usize,
) -> Result<Vec<RVal>, String> {
    let base_m = initial_machine(program)?;
    let (mut curr_val, mut curr_trace) = run_trace(base_m.fork(), rng, None, &HashMap::new())?;

    let mut chain = Vec::with_capacity(steps);

    for i in 0..(steps + warmup) {
        let mut addresses: Vec<Addr> = curr_trace.values.keys().cloned().collect();
        addresses.sort();

        if addresses.is_empty() {
            if i >= warmup {
                chain.push(curr_val.clone());
            }
            continue;
        }

        let a0_idx = rng.random_range(0..addresses.len());
        let a0 = &addresses[a0_idx];

        let (prop_val, prop_trace) = run_trace(base_m.fork(), rng, Some(a0), &curr_trace.values)?;

        let log_alpha = mh_log_alpha(&curr_trace, &prop_trace, a0);
        let u: f64 = rng.random();

        if log_alpha >= 0.0 || u.ln() < log_alpha {
            curr_val = prop_val;
            curr_trace = prop_trace;
        }

        if i >= warmup {
            chain.push(curr_val.clone());
        }
    }
    Ok(chain)
}

fn mh_log_alpha(curr: &Trace, prop: &Trace, a0: &Addr) -> f64 {
    let num_s: f64 = prop
        .sample_log_probs
        .iter()
        .filter(|&(k, _)| k != a0 && curr.values.contains_key(k))
        .map(|(_, p)| p)
        .sum();

    let den_s: f64 = curr
        .sample_log_probs
        .iter()
        .filter(|&(k, _)| k != a0 && prop.values.contains_key(k))
        .map(|(_, p)| p)
        .sum();

    let num = num_s + prop.observe_log_probs.values().sum::<f64>();
    let den = den_s + curr.observe_log_probs.values().sum::<f64>();

    let len_diff = (curr.values.len() as f64).ln() - (prop.values.len() as f64).ln();

    len_diff + (num - den)
}

fn run_trace<R: Rng + ?Sized>(
    mut m: Machine,
    rng: &mut R,
    x0: Option<&Addr>,
    cache: &HashMap<Addr, RVal>,
) -> Result<(RVal, Trace), String> {
    let mut trace = Trace::default();

    loop {
        match resume(m)? {
            Msg::Sample(a, dist, mut next_m) => {
                let x = if Some(&a) == x0 {
                    dist.sample(rng)
                } else if let Some(cached_val) = cache.get(&a) {
                    cached_val.clone()
                } else {
                    dist.sample(rng)
                };

                let lp = dist.log_prob(&x);

                trace.values.insert(a.clone(), x.clone());
                trace.sample_log_probs.insert(a, lp);

                send(&mut next_m, x);
                m = next_m;
            }
            Msg::Observe(addr, dist, y_obs, mut next_m) => {
                let lp = dist.log_prob(&y_obs);

                trace.observe_log_probs.insert(addr, lp);

                send(&mut next_m, y_obs);
                m = next_m;
            }
            Msg::Done(value, _) => return Ok((value, trace)),
        }
    }
}
