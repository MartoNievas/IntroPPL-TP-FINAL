/*

Módulo que implementa el algoritmo Black-Box Variational Inference (BBVI).
Convierte la inferencia en un problema de optimización, ajustando los parámetros
de una distribución guía q(x; theta) para maximizar la ELBO (Evidence Lower Bound)
utilizando el estimador de Score Function (REINFORCE).

*/

use std::collections::HashMap;
use crate::interpreter::{initial_machine, resume, send, Addr, Machine, Msg};
use crate::parser::distribution::{make_guide, make_distribution, Distribution};
use crate::parser::value::RVal;
use rand::prelude::*;
use rand_distr::num_traits::{Float, Pow};


/// Estructura interna para almacenar el resultado de una traza muestreada desde la guía.
struct SampleResult {
    pub val: RVal,
    pub elbo_sample: f64,
    pub scores: HashMap<Addr, Vec<f64>>,
}

// Optimizacion Adam para ascenso de gradiente (maximizar el ELBO)
struct AdamOptimizer {
    m: HashMap<Addr, Vec<f64>>,
    v: HashMap<Addr, Vec<f64>>,
    beta1: f64,
    beta2: f64,
    eps: f64,
    lr: f64,
    t: usize,
}


// Ejecuta el algoritmo Black-Box Variational Inference.
// Retorna el historial de convergencia de la ELBO y los parámetros variacionales optimizados θ.
pub fn run_bbvi<R: Rng + ?Sized>(
    program: &str,
    steps: usize,
    n_samples: usize, // Muestras por paso de gradiente para reducir varianza (e.g., 10 a 20)
    lr: f64,          // Tasa de aprendizaje para Adam (e.g., 0.05)
    rng: &mut R,
) -> Result<(Vec<f64>, HashMap<Addr, Vec<f64>>), String> {
    if n_samples == 0 {
        return Err("BBVI Error: 'n_samples' (batch size per optimization step) must be strictly greater than 0.".into());
    }

    let base_m = initial_machine(program)?;
    let mut guides: HashMap<Addr, Distribution> = HashMap::new();
    let mut theta: HashMap<Addr, Vec<f64>> = HashMap::new();
    let mut elbo_history = Vec::with_capacity(steps);
    let mut optimizer = AdamOptimizer::new(lr);

    for _step in 0..steps {
        let mut step_elbos = Vec::with_capacity(n_samples);
        let mut step_scores = Vec::with_capacity(n_samples);

        // Recolectamos N trazas independientes muestreando de las guías actuales
        for _ in 0..n_samples {
            let res = run_bbvi_sample(base_m.fork(), &mut guides, &mut theta, rng)?;
            step_elbos.push(res.elbo_sample);
            step_scores.push(res.scores);
        }

        // Calculamos la ELBO media del lote (la cota que queremos maximizar)
        let mean_elbo: f64 = step_elbos.iter().sum::<f64>() / (n_samples as f64);
        elbo_history.push(mean_elbo);

        // Reducción de varianza (Baseline): Usamos la ELBO media como control variate 'b'
        let mut grad_accum: HashMap<Addr, Vec<f64>> = HashMap::new();

        for (elbo_i, scores_i) in step_elbos.iter().zip(step_scores.iter()) {
            // Recompensa centrada: (w_i - b) reduce drásticamente el ruido del gradiente
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

        // Actualizamos los parámetros θ usando Adam
        optimizer.step(&mut theta, &grad_accum);
    }

    Ok((elbo_history, theta))
}

impl AdamOptimizer {
    fn new(lr: f64) -> Self {
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

    // Realiza un paso de ascendo de gradiente (theta_new = theta_old + lr * Adam(∇ELBO)).
    fn step(&mut self, theta: &mut HashMap<Addr, Vec<f64>>, grads: &HashMap<Addr, Vec<f64>>) {
        self.t += 1;
        let t_f64 = self.t as f64;

        // Corregimos el sesgo
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


                // Aqui hacemos el ascenso de grandiente
                params[k] += lr_t * m_vec[k] / (v_vec[k].sqrt() + self.eps);
            }

        }

    }

}


// Ejecuta una sola trayectoria del programa muestreando de las distribuciones guia q(x, theta)
fn run_bbvi_sample<R: Rng + ?Sized> (
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

                // Instanciamos q(x, theta) con los parametros actuales

                let guide_dist = guide_template.with_params(current_params).ok_or_else(|| {
                    format!("BBVI Error: Failed to instantiate varational guide distribution '{}' with parameters '{:?}' at address '{:?}'",guide_template.name(), current_params, addr)
                })?;

                // Ahora muestreamos de la GUIA q(x, theta), NO del prior
                let x = guide_dist.sample(rng);

                // Acumulamos probabilidades logaritmicas para el calculo del ELBO
                log_p += prior_dist.log_prob(&x);
                log_q += guide_dist.log_prob(&x);

                // Calculamos el gradiente de la Score Function: ∇θ log q(x; θ)
                if let Some(grad) = guide_dist.grad_log_prob(&x) {
                    scores.insert(addr.clone(), grad);
                } else {
                    return Err(format!("BBVI Error: Could not compute Score Function log-prob gradient for distribution '{}' at address '{:?}'.", guide_dist.name(), addr));
                }

                send(&mut next_m, x);
                m = next_m;

            }

            Msg::Observe(addr, dist, y_obs, mut next_m) => {
                log_p += dist.log_prob(&y_obs);
                send(&mut next_m, y_obs);
                m = next_m;
            }

            Msg::Done(val, _finish_machine) => {
                // ELBO para esta muestra: log p(x, y) - log q(x)
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