/*

Module that implements the Black-Box Variational Inference (BBVI) algorithm.
Turns inference into an optimization problem, adjusting the parameters of a
guide distribution q(x; theta) to maximize the ELBO (Evidence Lower Bound)
using the Score Function estimator (REINFORCE).

*/

use std::collections::HashMap;
use crate::interpreter::{initial_machine, resume, send, Addr, Machine, Msg};
use crate::parser::distribution::{make_guide, Distribution};
use crate::parser::value::RVal;
use rand::prelude::*;
use rand_distr::num_traits::{Pow};


/// Internal structure to store the result of a trace sampled from the guide.
pub(crate) struct SampleResult {
    pub val: RVal,
    pub elbo_sample: f64,
    pub scores: HashMap<Addr, Vec<f64>>,
}

// Adam optimizer for gradient ascent (maximizing the ELBO)
pub(crate) struct AdamOptimizer {
    m: HashMap<Addr, Vec<f64>>,
    v: HashMap<Addr, Vec<f64>>,
    beta1: f64,
    beta2: f64,
    eps: f64,
    lr: f64,
    t: usize,
}


// Runs the Black-Box Variational Inference algorithm.
// Returns the ELBO convergence history, the optimized variational
// parameters θ, and a posterior-predictive sample batch: the program's
// return value on each of the last step's traces, drawn from the final
// (optimized) guide -- this is BBVI's analogue of the sample population
// the other algorithms report a posterior estimate from.
pub fn run_bbvi<R: Rng + ?Sized>(
    program: &str,
    steps: usize,
    n_samples: usize, // Samples per gradient step to reduce variance (e.g., 10 to 20)
    lr: f64,          // Learning rate for Adam (e.g., 0.05)
    rng: &mut R,
) -> Result<(Vec<f64>, HashMap<Addr, Vec<f64>>, Vec<RVal>), String> {
    if n_samples == 0 {
        return Err("BBVI Error: 'n_samples' (batch size per optimization step) must be strictly greater than 0.".into());
    }

    let base_m = initial_machine(program)?;
    let mut guides: HashMap<Addr, Distribution> = HashMap::new();
    let mut theta: HashMap<Addr, Vec<f64>> = HashMap::new();
    let mut elbo_history = Vec::with_capacity(steps);
    let mut optimizer = AdamOptimizer::new(lr);
    let mut last_batch_vals = Vec::with_capacity(n_samples);

    for _step in 0..steps {
        let mut step_elbos = Vec::with_capacity(n_samples);
        let mut step_scores = Vec::with_capacity(n_samples);
        last_batch_vals.clear();

        // Collect N independent traces by sampling from the current guides
        for _ in 0..n_samples {
            let res = run_bbvi_sample(base_m.fork(), &mut guides, &mut theta, rng)?;
            step_elbos.push(res.elbo_sample);
            step_scores.push(res.scores);
            last_batch_vals.push(res.val);
        }

        // Compute the batch mean ELBO (the bound we want to maximize)
        let mean_elbo: f64 = step_elbos.iter().sum::<f64>() / (n_samples as f64);
        elbo_history.push(mean_elbo);

        // Variance reduction (baseline): use the mean ELBO as control variate 'b'
        let mut grad_accum: HashMap<Addr, Vec<f64>> = HashMap::new();

        for (elbo_i, scores_i) in step_elbos.iter().zip(step_scores.iter()) {
            // Centered reward: (w_i - b) drastically reduces gradient noise
            let reward = elbo_i - mean_elbo;

            for (addr, grad_i) in scores_i {
                let acc = grad_accum
                    .entry(addr.clone())
                    .or_insert_with(|| vec![0.0; grad_i.len()]);
                for (k, &g_val) in grad_i.iter().enumerate() {
                    acc[k] += reward * g_val / (n_samples as f64);
                }
            }
        }

        // Update the θ parameters using Adam
        optimizer.step(&mut theta, &grad_accum);
    }

    Ok((elbo_history, theta, last_batch_vals))
}

impl AdamOptimizer {
    pub(crate) fn new(lr: f64) -> Self {
        AdamOptimizer {
            m: HashMap::new(),
            v: HashMap::new(),
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            lr,
            t: 0
        }
    }

    // Performs a gradient ascent step (theta_new = theta_old + lr * Adam(∇ELBO)).
    pub(crate) fn step(&mut self, theta: &mut HashMap<Addr, Vec<f64>>, grads: &HashMap<Addr, Vec<f64>>) {
        self.t += 1;
        let t_f64 = self.t as f64;

        // Bias correction
        let lr_t = self.lr * ((1.0 - self.beta2.powf(t_f64)).sqrt()) / (1.0 - self.beta1.pow(t_f64));

        for (addr, grad) in grads {
            let params = match theta.get_mut(addr) {
                Some(p) => p,
                None => continue,
            };

            let m_vec = self.m.entry(addr.clone()).or_insert_with(|| vec![0.0; params.len()]);
            let v_vec = self.v.entry(addr.clone()).or_insert_with(|| vec![0.0; params.len()]);

            for k in 0..params.len() {
                let g = grad[k];
                m_vec[k] = self.beta1 * m_vec[k] + (1.0 - self.beta1) * g;
                v_vec[k] = self.beta2 * v_vec[k] + (1.0 - self.beta2) * g * g;


                // Here we perform the gradient ascent step
                params[k] += lr_t * m_vec[k] / (v_vec[k].sqrt() + self.eps);
            }

        }

    }

}


// Runs a single trajectory of the program, sampling from the guide distributions q(x, theta)
pub(crate) fn run_bbvi_sample<R: Rng + ?Sized> (
    mut m: Machine,
    guides: &mut HashMap<Addr, Distribution>,
    theta: &mut HashMap<Addr, Vec<f64>>,
    rng: &mut R,
) -> Result<SampleResult, String> {
    let mut log_p = 0.0;
    let mut log_q = 0.0;
    let mut scores : HashMap<Addr, Vec<f64>> = HashMap::new();

    loop {

        match resume(m)? {
            Msg::Sample(addr,prior_dist , mut next_m ) => {
                if !guides.contains_key(&addr) {
                    let guide = make_guide(&prior_dist)?;
                    let init_params = guide.params().ok_or_else(|| {
                        format!("BBVI Error: The distribution family '{}' at address '{:?}' does not support continuous parameter optimization.",guide.name(), addr)
                    })?;
                    
                    guides.insert(addr.clone(), guide);
                    theta.insert(addr.clone(), init_params);
                }

                let guide_template = guides.get(&addr).unwrap();
                let current_params = theta.get(&addr).unwrap();

                // Instantiate q(x, theta) with the current parameters

                let guide_dist = guide_template.with_params(current_params).ok_or_else(|| {
                    format!("BBVI Error: Failed to instantiate varational guide distribution '{}' with parameters '{:?}' at address '{:?}'",guide_template.name(), current_params, addr)
                })?;

                // Now we sample from the GUIDE q(x, theta), NOT from the prior
                let x = guide_dist.sample(rng);

                // Accumulate log-probabilities for the ELBO computation
                log_p += prior_dist.log_prob(&x);
                log_q += guide_dist.log_prob(&x);

                // Compute the Score Function gradient: ∇θ log q(x; θ)
                if let Some(grad) = guide_dist.grad_log_prob(&x) {
                    scores.insert(addr.clone(), grad);
                } else {
                    return Err(format!("BBVI Error: Could not compute Score Function log-prob gradient for distribution '{}' at address '{:?}'.", guide_dist.name(), addr));
                }

                send(&mut next_m, x);
                m = next_m;

            }

            Msg::Factor(_addr, val , mut next_m  ) => {
                log_p += val;
                send(&mut next_m, RVal::Nil);
                m = next_m;
            }

            Msg::Observe(_addr, dist, y_obs, mut next_m) => {
                log_p += dist.log_prob(&y_obs);
                send(&mut next_m, y_obs);
                m = next_m;
            }

            Msg::Done(val, _finish_machine) => {
                let elbo_sample = log_p - log_q;
                return Ok(SampleResult {
                    val,
                    elbo_sample,
                    scores,
                });
             }
        }

    }
}