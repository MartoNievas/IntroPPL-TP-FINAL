/* 

Objeto tipo distribución, que contiene la información de la distribución de un conjunto de datos.

Todas las distrbuciones soportan las operaciones:
    - sample: devuelve un valor aleatorio de la distribución
    - log-prob: devuelve el logaritmo de la probabilidad de un valor dado

*/

use core::f64;

use rand::prelude::*;
use rand_distr::Distribution as RandDistribution;
use statrs::function::gamma::ln_gamma as lgamma;
use rand_distr::{
    Bernoulli as RBernoulli,
    Beta as RBeta,
    Distribution as RandDistr, // trait .sample() de rand_distr; alias para no chocar
    Exp as RExp,               // con el enum `Distribution`
    Gamma as RGamma,
    LogNormal as RLogNormal,
    Normal as RNormal,
    Poisson as RPoisson,
    Uniform as RUniform,
    multi::Dirichlet as RDirichlet,
    weighted::WeightedIndex,
};

// Definición de constante en Rust
const LOG2PI: f64 = 1.8378770664093453;

#[derive(Debug, Clone)]
pub enum Distribution {
    Normal { mu: f64, sigma: f64 },
    LogNormal { mu: f64, sigma: f64 },
    Uniform { a: f64, b: f64 }, // uniform-continuous
    Exponential { rate: f64 },
    Beta { alpha: f64, beta: f64 },
    Gamma { shape: f64, rate: f64 }, // shape/rate, como en el libro
    Poisson { lam: f64 },
    Bernoulli { p: f64 }, // "flip"
    Discrete { probs: Vec<f64> }, // categorical sobre {0..K-1}, ya normalizado
    UniformDiscrete { lo: i64, hi: i64 }, // enteros en [lo, hi)
    Dirichlet { alphas: Vec<f64> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(f64),     // normal, log-normal, uniform-continuous, exponential, beta, gamma
    Integer(i64),    // poisson, discrete (índice), uniform-discrete
    Boolean(bool),   // bernoulli / flip
    Vector(Vec<f64>),// dirichlet
}

pub type SampleResult = Value;

// ---------------------------------------------------------------------------
// Distribuciones y sus operaciones: sample, log_prob, params, with_params, grad_log_prob
// ---------------------------------------------------------------------------


impl Distribution {
    
    // Constructor functions for each distribution type, with validation of parameters

    pub fn normal(mu: f64, sigma: f64) -> Result<Self, String> {
        if sigma <= 0.0 {
            return Err("sigma must be positive".to_string());
        }
        Ok(Distribution::Normal { mu, sigma })
    }

    pub fn log_normal(mu: f64, sigma: f64) -> Result<Self, String> {
        if sigma <= 0.0 {
            return Err("sigma must be positive".to_string());
        }
        Ok(Distribution::LogNormal { mu, sigma })
    }

    pub fn uniform(a: f64, b: f64) -> Result<Self, String> {
        if a >= b {
            return Err("a must be less than b".to_string());
        }
        Ok(Distribution::Uniform { a, b })
    }

    pub fn exponential(rate: f64) -> Result<Self, String> {
        if rate <= 0.0 {
            return Err("rate must be positive".to_string());
        }
        Ok(Distribution::Exponential { rate })
    }

    pub fn beta(alpha: f64, beta: f64) -> Result<Self, String> {
        if alpha <= 0.0 || beta <= 0.0 {
            return Err("alpha and beta must be positive".to_string());
        }
        Ok(Distribution::Beta { alpha, beta })
    }

    pub fn gamma(shape: f64, rate: f64) -> Result<Self, String> {
        if shape <= 0.0 || rate <= 0.0 {
            return Err("shape and rate must be positive".to_string());
        }
        Ok(Distribution::Gamma { shape, rate })
    }

    pub fn poisson(lam: f64) -> Result<Self, String> {
        if lam <= 0.0 {
            return Err("lambda must be positive".to_string());
        }
        Ok(Distribution::Poisson { lam })
    }

    pub fn bernoulli(p: f64) -> Result<Self, String> {
        if !(0.0..=1.0).contains(&p) {
            return Err("flip: p must be in [0,1]".into());
        }
        Ok(Distribution::Bernoulli { p })
    }

    pub fn discrete(probs: &[f64]) -> Result<Self, String> {
        if probs.iter().any(|&p| p < 0.0) || probs.iter().sum::<f64>() <= 0.0 {
            return Err("discrete: invalid probability vector".into());
        }
        let total: f64 = probs.iter().sum();
        Ok(Distribution::Discrete {
            probs: probs.iter().map(|p| p / total).collect(),
        })
    }

    pub fn uniform_discrete(lo: i64, hi: i64) -> Result<Self, String> {
        if lo >= hi {
            return Err("uniform-discrete: lo must be less than hi".into());
        }
        Ok(Distribution::UniformDiscrete { lo, hi })
    }

    pub fn dirichlet(alphas: &[f64]) -> Result<Self, String> {
        if alphas.iter().any(|&a| a <= 0.0) {
            return Err("dirichlet: all alphas must be positive".into());
        }
        Ok(Distribution::Dirichlet {
            alphas: alphas.to_vec(),
        })
    }
}    
    // Nombres de primitivos de distribuciones, para mostrar en errores y logs

impl Distribution {
    pub fn name(&self) -> &'static str {
        match self {
            Distribution::Normal { .. } => "normal",
            Distribution::LogNormal { .. } => "log-normal",
            Distribution::Uniform { .. } => "uniform-continuous",
            Distribution::Exponential { .. } => "exponential",
            Distribution::Beta { .. } => "beta",
            Distribution::Gamma { .. } => "gamma",
            Distribution::Poisson { .. } => "poisson",
            Distribution::Bernoulli { .. } => "flip",
            Distribution::Discrete { .. } => "discrete",
            Distribution::UniformDiscrete { .. } => "uniform-discrete",
            Distribution::Dirichlet { .. } => "dirichlet",
        }
    }
}

// SAMPLES: 

impl Distribution {
    
    // samples un valor aleatorio de la distribución, usando un generador de números aleatorios `rng`.

    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SampleResult {
        match self {
            Distribution::Normal { mu, sigma } => {
                let normal = RNormal::new(*mu, *sigma).unwrap();
                Value::Scalar(normal.sample(rng))
            }
            Distribution::LogNormal { mu, sigma } => {
                let log_normal = RLogNormal::new(*mu, *sigma).unwrap();
                Value::Scalar(log_normal.sample(rng))
            }
            Distribution::Uniform { a, b } => {
                let uniform = RUniform::new(*a, *b).unwrap();
                Value::Scalar(uniform.sample(rng))
            }
            Distribution::Exponential { rate } => {
                let exp = RExp::new(*rate).unwrap();
                Value::Scalar(exp.sample(rng))
            }
            Distribution::Beta { alpha, beta } => {
                let beta = RBeta::new(*alpha, *beta).unwrap();
                Value::Scalar(beta.sample(rng))
            }
            Distribution::Gamma { shape, rate } => {
                let gamma = RGamma::new(*shape, *rate).unwrap();
                Value::Scalar(gamma.sample(rng))
            }
            Distribution::Poisson { lam } => {
                let poisson = RPoisson::new(*lam).unwrap();
                Value::Integer(poisson.sample(rng) as i64)
            }
            Distribution::Bernoulli { p } => {
                let bernoulli = RBernoulli::new(*p).unwrap();
                Value::Boolean(bernoulli.sample(rng))
            }
            Distribution::Discrete { probs } => {
                let dist = WeightedIndex::new(probs).unwrap();
                Value::Integer(dist.sample(rng) as i64)
            }
            Distribution::UniformDiscrete { lo, hi } => {
                let uniform_discrete = RUniform::new(*lo, *hi).unwrap();
                Value::Integer(uniform_discrete.sample(rng))
            }
            Distribution::Dirichlet { alphas } => {
                let dist = RDirichlet::new(&alphas.clone()).unwrap();
                Value::Vector(dist.sample(rng))
            }

    }

}
}

// LOG-PROBABILITIES

impl Distribution {
    pub fn log_prob(&self, x: &Value) -> f64 {
        match (self, x) {
            (Distribution::Normal { mu, sigma }, Value::Scalar(x)) => {
                let z = (x - mu) / sigma;
                -0.5 * z * z - (sigma.ln() + 0.5 * LOG2PI)
            }
            (Distribution::LogNormal { mu, sigma }, Value::Scalar(x)) => {
                if *x <= 0.0 {
                    f64::NEG_INFINITY
                } else {
                    let log_x = x.ln();
                    let z = (log_x - mu) / sigma;
                    -0.5 * z * z - (sigma.ln() + log_x + 0.5 * LOG2PI)
                }
            }
            (Distribution::Uniform { a, b }, Value::Scalar(x)) => {
                if *a < *x && *x < *b {
                    -(b - a).ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            (Distribution::Exponential { rate}, Value::Scalar(x)) => {
                if *x < 0.0 {
                    f64::NEG_INFINITY
                } else {
                    rate.ln() - rate * x
                }
            }
            (Distribution::Beta { alpha, beta }, Value::Scalar(x)) => {
                if !(0.0 < *x && *x < 1.0) {
                    f64::NEG_INFINITY
                } else {
                    let log_beta = lgamma(*alpha) + lgamma(*beta) - lgamma(alpha + beta);
                    (alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln() - log_beta
                }
            }
            (Distribution::Gamma { shape, rate}, Value::Scalar(x)) => {
                if *x <= 0.0 {
                    f64::NEG_INFINITY
                } else {
                    shape * rate.ln() - lgamma(*shape) + (shape - 1.0) * x.ln() - rate * x
                }
            }
            (Distribution::Poisson { lam },Value::Integer(k)) => {
                if *k < 0 {
                    f64::NEG_INFINITY
                } else {
                    *k as f64 * lam.ln() - lam - lgamma((*k + 1) as f64)
                }
            }
            (Distribution::Bernoulli { p },Value::Boolean(b)) => {
                if *b {
                    if *p > 0.0 { p.ln() } else { f64::NEG_INFINITY }
                } else {
                    if *p < 1.0 { (1.0 - *p).ln() } else { f64::NEG_INFINITY }
                }
            }
            (Distribution::Discrete { probs }, Value::Integer(k)) => {
                let k = *k;
                if k >= 0 && (k as usize) < probs.len() && probs[k as usize] > 0.0 {
                    probs[k as usize].ln()
                } else {
                    f64::NEG_INFINITY
                }
            }

            (Distribution::UniformDiscrete { lo, hi }, Value::Integer(k)) => {
                if *lo <= *k && *k < *hi {
                    -((*hi - *lo) as f64).ln()
                } else {
                    f64::NEG_INFINITY
                }
            }
            (Distribution::Dirichlet { alphas }, Value::Vector(x_vec)) => {

                if x_vec.len() != alphas.len() || x_vec.iter().any(|&val| val <= 0.0) {
                    return f64::NEG_INFINITY;
                }

                // 2. logB (normalización de la distribución)
                // logB = sum(lgamma(a_i)) - lgamma(sum(a_i))
                let sum_lgamma_alphas: f64 = alphas.iter().map(|&a| lgamma(a)).sum();
                let lgamma_sum_alphas = lgamma(alphas.iter().sum());
                let log_b = sum_lgamma_alphas - lgamma_sum_alphas;

                // 3. log-likelihood
                // sum((a_i - 1) * ln(x_i)) - logB
                let log_likelihood: f64 = alphas.iter()
                    .zip(x_vec.iter())
                    .map(|(&a, &x)| (a - 1.0) * x.ln())
                    .sum();

                log_likelihood - log_b
            }
            _ => f64::NEG_INFINITY, // incompatible value type for the distribution
        }

    }
}

// --- interfaz de "guide" para BBVI (params / with_params / grad_log_prob) --

impl Distribution {
    // Funcion que devuelve los parámetros de la distribución como un vector de f64, si es aplicable. Para distribuciones que no tienen parámetros (como Bernoulli con p fijo), devuelve None.
    pub fn params(&self) -> Option<Vec<f64>> {
        match self {
            Distribution::Normal { mu, sigma } => Some(vec![*mu, sigma.ln()]),
            Distribution::LogNormal { mu, sigma } => Some(vec![*mu, sigma.ln()]),
            Distribution::Bernoulli { p } => Some(vec![(*p / (1.0 - *p)).ln()]), // logit(p)
            Distribution::Discrete { probs } => {
                Some(probs.iter().map(|&p| p.max(1e-10).ln()).collect()) // log-probs, evitando log(0)
            }
            _ => None, // otras distribuciones no tienen parámetros continuos
        }
    }

        /// Nueva instancia a partir de un vector de parámetros no restringidos.
    pub fn with_params(&self, theta: &[f64]) -> Option<Distribution> {
        match self {
            Distribution::Normal { .. } => Some(Distribution::Normal {
                mu: theta[0],
                sigma: theta[1].exp(),
            }),
            Distribution::LogNormal { .. } => Some(Distribution::LogNormal {
                mu: theta[0],
                sigma: theta[1].exp(),
            }),
            Distribution::Bernoulli { .. } => Some(Distribution::Bernoulli {
                p: sigmoid(theta[0]),
            }),
            Distribution::Discrete { .. } => Some(Distribution::Discrete {
                probs: softmax(theta),
            }),
            _ => None,
        }
    }

    /// Gradiente de log_prob(x) respecto de los parámetros no restringidos.
    pub fn grad_log_prob(&self, x: &Value) -> Option<Vec<f64>> {
        match (self, x) {
            (Distribution::Normal { mu, sigma }, Value::Scalar(x)) => {
                let z = (x - mu) / sigma;
                Some(vec![z / sigma, z * z - 1.0])
            }
            (Distribution::LogNormal { mu, sigma }, Value::Scalar(x)) => {
                let z = (x.ln() - mu) / sigma;
                Some(vec![z / sigma, z * z - 1.0])
            }
            (Distribution::Bernoulli { p }, Value::Boolean(b)) => {
                let indicator = if *b { 1.0 } else { 0.0 };
                Some(vec![indicator - p])
            }
            (Distribution::Discrete { probs }, Value::Integer(k)) => {
                let mut onehot = vec![0.0; probs.len()];
                onehot[*k as usize] = 1.0;
                Some(onehot.iter().zip(probs.iter()).map(|(o, p)| o - p).collect())
            }
            _ => None,
        }
    }

}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
 
fn softmax(v: &[f64]) -> Vec<f64> {
    let m = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = v.iter().map(|x| (x - m).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.into_iter().map(|e| e / sum).collect()
}


// Tabla de constructores de distribuciones (Nombre primitivos visibles)

pub fn make_distribution(name: &str, args: &[f64]) -> Result<Distribution, String> {
    match name {
        "normal" => Distribution::normal(args[0], args[1]),
        "log-normal" => Distribution::log_normal(args[0], args[1]),
        "uniform-continuous" | "uniform" => Distribution::uniform(args[0], args[1]),
        "exponential" => Distribution::exponential(args[0]),
        "beta" => Distribution::beta(args[0], args[1]),
        "gamma" => Distribution::gamma(args[0], args[1]),
        "poisson" => Distribution::poisson(args[0]),
        "flip" | "bernoulli" => Distribution::bernoulli(args[0]),
        "discrete" | "categorical" => Distribution::discrete(&args.to_vec()),
        "uniform-discrete" => Distribution::uniform_discrete(args[0] as i64, args[1] as i64),
        "dirichlet" => Distribution::dirichlet(&args.to_vec()),
        _ => Err(format!("Unknown distribution name: {}", name)),
    }
}

// Guide variacional optimizable con el mismo soporte que `d` (BBVI).

pub fn make_guide(d: &Distribution) -> Result<Distribution, String> {
    match d {
        Distribution::Normal { mu, sigma } => Distribution::normal(*mu, *sigma),
        Distribution::LogNormal { mu, sigma } => Distribution::log_normal(*mu, *sigma),
                Distribution::Gamma { .. } | Distribution::Exponential { .. } | Distribution::Beta { .. } => {
            // soporte positivo -> inicialización tipo log-normal
            Distribution::log_normal(0.0, 1.0)
        }
        Distribution::Bernoulli { p } => Distribution::bernoulli(*p),
        Distribution::Discrete { probs } => Distribution::discrete(&probs.to_vec()),
        other => Err(format!("no optimizable guide family for distribution: {}", other.name())),  
    }
}

// --- repr legible, equivalente a __repr__ del Python ----------------------
 
impl std::fmt::Display for Distribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let params: Vec<String> = match self {
            Distribution::Normal { mu, sigma } => vec![mu.to_string(), sigma.to_string()],
            Distribution::LogNormal { mu, sigma } => vec![mu.to_string(), sigma.to_string()],
            Distribution::Uniform { a, b } => vec![a.to_string(), b.to_string()],
            Distribution::Exponential { rate } => vec![rate.to_string()],
            Distribution::Beta { alpha, beta } => vec![alpha.to_string(), beta.to_string()],
            Distribution::Gamma { shape, rate } => vec![shape.to_string(), rate.to_string()],
            Distribution::Poisson { lam } => vec![lam.to_string()],
            Distribution::Bernoulli { p } => vec![p.to_string()],
            Distribution::Discrete { probs } => vec![format!("{probs:?}")],
            Distribution::UniformDiscrete { lo, hi } => vec![lo.to_string(), hi.to_string()],
            Distribution::Dirichlet { alphas } => vec![format!("{alphas:?}")],
        };
        write!(f, "({} {})", self.name(), params.join(" "))
    }
}
