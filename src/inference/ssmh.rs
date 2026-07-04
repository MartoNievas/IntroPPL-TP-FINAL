/*

Modulo que implemente el algoritmo de inferencia Single Site Metropolis-Hastings / SSMH, este es un algoritmo Markov Chain Monte Carlo / MCMC
El mismo realiza una camina aleatoria sobre la traza de ejecucion.

*/

use std::collections::{HashMap, HashSet};
use crate::interpreter::{initial_machine, resume, send, Addr, Machine, Msg};
use crate::parser::value::RVal;
use rand::prelude::*;

/// La Traza captura el historial completo de una ejecución del programa.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub values: HashMap<Addr, RVal>,
    pub sample_log_probs: HashMap<Addr, f64>,
    pub observe_log_probs: HashMap<Addr, f64>,
}

fn mh_log_alpha(curr: &Trace, prop: &Trace, a0: &Addr) -> f64 {
    let num_s : f64 = prop.sample_log_probs.iter()
        .filter(|&(k, _)| k != a0 && curr.values.contains_key(k))
        .map(|(_, p)| p)
        .sum();

    let den_s: f64 = curr.sample_log_probs.iter()
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
            },
            Msg::Observe(addr, dist, y_obs, mut next_m) => {
                let obs_val = RVal::Float(y_obs);
                let lp = dist.log_prob(&obs_val);

                trace.observe_log_probs.insert(addr, lp);

                send(&mut next_m, obs_val);
                m = next_m;
            },
            Msg::Done(value, _) => {
                return Ok((value, trace))
            },
            
        }
    }
}

/*

def single_site_mh(program, rng, steps, warmup=2000):
    value, X, S, O = run(program, rng, None, {})    # traza inicial: nada que remuestrear o reutilizar
    chain = []
    for i in range(steps + warmup):
        a0 = list(X)[int(rng.integers(len(X)))]     # elegir una dirección/sitio para cambiar
        
        # 1. Proponer: re-ejecutar con x0=a0, reutilizando la traza actual X como caché
        val2, X2, S2, O2 = run(program, rng, a0, X)
        
        # 2. Calcular el ratio de aceptación en escala logarítmica
        log_alpha = mh_log_alpha(X, X2, S, S2, O, O2, a0)
        
        # 3. Paso de aceptación/rechazo (si ln(u) < log_alpha, aceptamos la propuesta)
        if np.log(rng.uniform()) < log_alpha:
            value, X, S, O = val2, X2, S2, O2
            
        if i >= warmup:
            chain.append(float(value))
    return np.array(chain, dtype=float)

*/


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
        let addresses: Vec<Addr> = curr_trace.values.keys().cloned().collect();

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