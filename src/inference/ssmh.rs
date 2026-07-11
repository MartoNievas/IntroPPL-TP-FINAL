/*

Module that implements the Single-Site Metropolis-Hastings (SSMH) inference
algorithm, a Markov Chain Monte Carlo (MCMC) algorithm. It performs a random
walk over the program's execution trace.

In addition to the sample chain, `single_site_mh` also reports the
acceptance rate of the Metropolis-Hastings proposals: the canonical MCMC
diagnostic for whether the random walk is exploring the posterior at a
reasonable pace (too low means the chain barely moves; too high usually
means the proposal is too conservative and under-explores the space).

*/

use crate::interpreter::{Addr, Machine, Msg, initial_machine, resume, send};
use crate::parser::value::RVal;
use rand::prelude::*;
use std::collections::HashMap;

/// The Trace captures the complete history of a single execution of the program.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub values: HashMap<Addr, RVal>,
    pub sample_log_probs: HashMap<Addr, f64>,
    pub observe_log_probs: HashMap<Addr, f64>,
    // Accumulated log-weight coming from `factor` calls during this run.
    // Unlike observe_log_probs, this is not keyed by address: `factor` does
    // not pause the machine, so there's no Msg to intercept per call. We
    // read it straight from the machine's `log_w` once the trace reaches
    // Done, since `FactorK` already accumulated it there internally.
    pub factor_log_w: f64,
}

/// Runs SSMH for `steps` post-warmup samples (plus `warmup` discarded
/// samples). Returns the resulting chain together with the overall
/// acceptance rate of the Metropolis-Hastings proposals (accepted /
/// attempted, across both warmup and post-warmup iterations).
///
/// If the model has no probabilistic `sample` sites at all (so there is
/// never anything to propose a change to), the acceptance rate is reported
/// as `f64::NAN` rather than a division by zero -- callers should check
/// `is_nan()` before formatting it.
pub fn single_site_mh<R: Rng + ?Sized>(
    program: &str,
    rng: &mut R,
    steps: usize,
    warmup: usize,
) -> Result<(Vec<RVal>, f64), String> {
    let base_m = initial_machine(program)?;
    let (mut curr_val, mut curr_trace) = run_trace(base_m.fork(), rng, None, &HashMap::new())?;

    let mut chain = Vec::with_capacity(steps);
    let mut accepted: usize = 0;
    let mut attempted: usize = 0;

    for i in 0..(steps + warmup) {
        let mut addresses: Vec<Addr> = curr_trace.values.keys().cloned().collect();
        addresses.sort();

        if addresses.is_empty() {
            if i >= warmup {
                chain.push(curr_val.clone());
            }
            continue;
        }

        attempted += 1;

        let a0_idx = rng.random_range(0..addresses.len());
        let a0 = &addresses[a0_idx];

        let (prop_val, prop_trace) = run_trace(base_m.fork(), rng, Some(a0), &curr_trace.values)?;

        let log_alpha = mh_log_alpha(&curr_trace, &prop_trace, a0);
        let u: f64 = rng.random();

        if log_alpha >= 0.0 || u.ln() < log_alpha {
            curr_val = prop_val;
            curr_trace = prop_trace;
            accepted += 1;
        }

        if i >= warmup {
            chain.push(curr_val.clone());
        }
    }

    let acceptance_rate = if attempted > 0 {
        accepted as f64 / attempted as f64
    } else {
        // Nothing was ever proposed on (e.g. a model with no 'sample'
        // sites), so there is no meaningful accept/reject ratio to report.
        f64::NAN
    };

    Ok((chain, acceptance_rate))
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

    // factor_log_w is added just like observe_log_probs: both represent
    // density the trace accumulates without generating a new random
    // choice, so they enter the ratio without needing a proposal term.
    let num = num_s + prop.observe_log_probs.values().sum::<f64>() + prop.factor_log_w;
    let den = den_s + curr.observe_log_probs.values().sum::<f64>() + curr.factor_log_w;

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
            Msg::Done(value, final_m) => {
                // At this point final_m.log_w can only hold whatever
                // factor() calls added during this run (observe doesn't
                // touch log_w in this algorithm, see above), so we store it
                // whole in the trace before discarding the machine.
                trace.factor_log_w = final_m.log_w;
                return Ok((value, trace));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn factor_shifts_posterior_mean() {
        // Same model as factor_test.hoppl: a factor that replicates
        // (observe (normal mu 1.0) 3.0) by hand. Without the factor_log_w
        // fix in mh_log_alpha, the chain never moves away from the prior
        // and this assertion fails.
        let model = r#"
            (let [mu (sample (normal 0.0 10.0))
                  diff (- mu 3.0)
                  log_lik (* -0.5 (* diff diff))]
                (factor log_lik)
                mu)
        "#;

        let mut rng = StdRng::seed_from_u64(42);
        let (chain, acceptance_rate) = single_site_mh(model, &mut rng, 4000, 1000).unwrap();

        let mean: f64 = chain
            .iter()
            .map(|v| match v {
                RVal::Float(f) => *f,
                RVal::Int(i) => *i as f64,
                other => panic!("non-numeric value in the chain: {other:?}"),
            })
            .sum::<f64>()
            / chain.len() as f64;

        assert!(
            (mean - 3.0).abs() < 0.5,
            "expected mean near 3.0, got {mean}"
        );
        assert!(
            acceptance_rate > 0.0 && acceptance_rate <= 1.0,
            "acceptance rate out of the (0, 1] range: {acceptance_rate}"
        );
    }

    #[test]
    fn acceptance_rate_is_nan_when_model_has_no_sample_sites() {
        // A model that only uses factor (or is fully deterministic) never
        // proposes a change to anything, so there's no accept/reject ratio.
        let model = "(factor 1.0)";
        let mut rng = StdRng::seed_from_u64(1);
        let (chain, acceptance_rate) = single_site_mh(model, &mut rng, 10, 0).unwrap();
        assert_eq!(chain.len(), 10);
        assert!(acceptance_rate.is_nan());
    }
}