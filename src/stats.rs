/*

Statistics and inference diagnostics module.

Takes the raw output from the inference engines (`RVal` values, importance
weights, MCMC chains) and computes the summary metrics: mean, standard
error, Effective Sample Size (ESS), and, for MCMC, ESS corrected for
autocorrelation (integrated autocorrelation time).

It knows nothing about the CLI, HOPPL parsing, or output formatting: it
receives vectors of values and returns numbers or frequency distributions.
If the model returns a non-numeric value (e.g. a string), it falls back to
a categorical summary instead of failing.

Cases covered:
    - LW: weighted mean/variance, ESS from importance weights.
    - SMC: sample mean and standard error (unweighted).
    - SSMH (MCMC): mean, standard error, and ESS adjusted for autocorrelation.
    - Non-numeric results: categorical frequency distribution, weighted
      or unweighted.

*/

use ppl_tp_final::parser::value::RVal;
use std::collections::HashMap;

use crate::ui::print_ok;

pub fn as_f64(val: &RVal) -> Result<f64,String> {
    match val {
        RVal::Float(f) => Ok(*f),
        RVal::Int(i) => Ok(*i as f64),
        _ => Err(format!("Expected a numeric value, got: {:?}",val)),
    }
}

pub fn is_numeric(val: &RVal) -> bool {
    matches!(val, RVal::Float(_) | RVal::Int(_))
}

pub fn print_categorical_weighted(vals: &[RVal], weights: &[f64]) {
    let mut mass: HashMap<String, f64> = HashMap::new();
    for (v, w) in vals.iter().zip(weights.iter()) {
        *mass.entry(v.to_string()).or_insert(0.0) += w;
    }
    let mut entries: Vec<(String, f64)> = mass.into_iter().collect();
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    print_ok("Non-numeric result: estimated posterior distribution (by weight):");
    for (val, p) in entries {
        println!("      {val}: {:.4}", p);
    }
}

pub fn print_categorical_unweighted(vals: &[RVal]) {
    let n = vals.len() as f64;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for v in vals {
        *counts.entry(v.to_string()).or_insert(0) += 1;
    }
    let mut entries: Vec<(String, usize)> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    print_ok("Non-numeric result: estimated posterior distribution (frequency):");
    for (val, c) in entries {
        println!("      {val}: {:.4} ({c}/{n})", c as f64 / n);
    }
}

pub fn weighted_mean_var(vals: &[RVal], weights: &[f64]) -> (f64, f64) {
    let mean: f64 = vals
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| as_f64(v).unwrap() * w)
        .sum();
    let var: f64 = vals
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| w * (as_f64(v).unwrap() - mean).powi(2))
        .sum();
    (mean, var)
}

pub fn effective_sample_size(weights: &[f64]) -> f64 {
    let sum_sq: f64 = weights.iter().map(|w| w * w).sum();
    1.0 / sum_sq
}

pub fn sample_mean_std_err(vals: &[RVal]) -> (f64, f64) {
    let n = vals.len() as f64;
    let mean: f64 = vals.iter().map(as_f64).map(|val| val.unwrap()).sum::<f64>() / n;
    let var: f64 = vals.iter().map(|x| (as_f64(x).unwrap() - mean).powi(2)).sum::<f64>() / n;
    let std_err = (var / n).sqrt();
    (mean, std_err)
}

fn autocorrelations(xs: &[f64], mean: f64, var: f64, max_lag: usize) -> Vec<f64> {
    let n = xs.len();
    let mut rhos = Vec::with_capacity(max_lag + 1);
    for k in 0..=max_lag {
        let cov: f64 = (0..n - k)
            .map(|i| (xs[i] - mean) * (xs[i + k] - mean))
            .sum::<f64>()
            / n as f64;
        rhos.push(cov / var);
    }
    rhos
}

fn integrated_autocorr_time(rhos: &[f64]) -> f64 {
    let mut sum_gamma = 0.0;
    let mut m = 1;
    loop {
        let idx1 = 2 * m - 1;
        let idx2 = 2 * m;
        if idx2 >= rhos.len() {
            break;
        }
        let gamma = rhos[idx1] + rhos[idx2];
        if gamma <= 0.0 {
            break;
        }
        sum_gamma += gamma;
        m += 1;
    }
    (1.0 + 2.0 * sum_gamma).max(1.0)
}

pub fn mcmc_mean_std_err_ess(chain: &[RVal]) -> (f64, f64, f64) {
    let xs: Vec<f64> = chain.iter().map(as_f64).map(|val| val.unwrap()).collect();
    let n = xs.len();
    let mean: f64 = xs.iter().sum::<f64>() / n as f64;
    let var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;

    if n < 4 || var == 0.0 {
        let std_err = (var / n as f64).sqrt();
        return (mean, std_err, n as f64);
    }

    let max_lag = (n / 2).min(1000);
    let rhos = autocorrelations(&xs, mean, var, max_lag);
    let tau = integrated_autocorr_time(&rhos);
    let ess = (n as f64 / tau).max(1.0);
    let std_err = (var / ess).sqrt();
    (mean, std_err, ess)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ess_uniform_weights_equals_n() {
        let w = vec![0.25; 4];
        assert!((effective_sample_size(&w) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn ess_degenerate_weight_equals_one() {
        let w = vec![1.0, 0.0, 0.0, 0.0];
        assert!((effective_sample_size(&w) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn as_f64_bool_mapping() {
        assert_eq!(as_f64(&RVal::Bool(true)), Err("Expected a numeric value, got: Bool(true)".into()));
        assert_eq!(as_f64(&RVal::Bool(false)), Err("Expected a numeric value, got: Bool(false)".into()));
    }

    #[test]
    fn is_numeric_rejects_str() {
        assert!(!is_numeric(&RVal::Str("large".to_string())));
        assert!(is_numeric(&RVal::Int(3)));
    }

    #[test]
    fn mcmc_diag_no_autocorrelation_matches_iid_var() {
        // Constant chain: variance 0, takes the short path (n < 4 or var == 0).
        let chain: Vec<RVal> = vec![RVal::Float(2.0); 10];
        let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);
        assert!((mean - 2.0).abs() < 1e-9);
        assert_eq!(std_err, 0.0);
        assert_eq!(ess, 10.0);
    }
}