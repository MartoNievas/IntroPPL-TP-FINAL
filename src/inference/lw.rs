/*

Modulo que implemente el algoritmo de inferencia Likelihood Weighting, el cual es el algoritmo de de infernecia mas elemental
El mismo ejecuta el programa sin pausar para cambiar trayectorias.

*/

use crate::interpreter::{initial_machine, resume, send, Msg};
use crate::parser::value::RVal;
use crate::parser::distribution::Distribution;
use rand::prelude::*;

pub fn run_lw<R: Rng + ?Sized>(program: &str, rng: &mut R) -> Result<(RVal, f64), String> {
    let mut m = initial_machine(program)?;


    loop {
        match resume(m.clone())? {
            Msg::Done(val, next_m) => {
                return Ok((val, next_m.log_w));
            }

            Msg::Sample(_addr, dist , mut next_m ) => {
                // 1. Muestreamos la distribucion prior
                let sample_val = dist.sample(rng);

                // 2. Inyectamos el valor y continuamos
                send(&mut next_m, sample_val);

                m = next_m;
            }

            Msg::Observe(_addr, dist, y_obs, mut next_m) => {
                // 1. Acumulamos el log-likelihood y continuamos
                next_m.log_w += dist.log_prob(&RVal::Float(y_obs));

                // 2 Inyectamos el valor observado para que el programa siga
                send(&mut next_m, RVal::Float(y_obs));

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
    

    // Corremos el programa N veces pasando el mismo &mut rng

    for _ in 0..n_particles {
        let (val, log_w) = run_lw(program, rng)?;
        values.push(val);
        log_weights.push(log_w);
    }

    // 1. Buscamos el log_weight maximo para evitar overflows/underflows
    let max_lw = log_weights
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // 2. Exponenciamos restando el maximo: exp(w_i - max_w)
    let exp_weights: Vec<f64> = log_weights
        .iter()
        .map(|&w| (w - max_lw).exp())
        .collect();

    // 3. Sumamos todos los pesos
    let sum_exp : f64 = exp_weights.iter().sum();

    let normalized_weights: Vec<f64> = exp_weights
        .iter()
        .map(|w| w / sum_exp)
        .collect();

    // Retornamos
    Ok((values, normalized_weights))
    
}