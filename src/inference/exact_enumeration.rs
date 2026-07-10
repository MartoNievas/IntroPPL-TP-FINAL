/*

Module that implements the Exact Enumeration inference algorithm.
This algorithm exhaustively explores every possible branch of a
probabilistic program with finite discrete variables.

*/

use crate::interpreter::{initial_machine, resume, send, Msg};
use crate::parser::value::RVal;
use std::collections::{HashMap};

// Mathematical helper function equivalent to Python's `scipy.special.logsumexp`.
// Computes log(exp(a) + exp(b)) in a numerically stable way.
fn log_add_exp(a: f64, b: f64) -> f64 {
    let m = a.max(b);
    if m == f64::NEG_INFINITY {
        m
    } else {
        m + ((a - m).exp() + (b - m).exp()).ln()
    }
}

/// Runs the Exact Enumeration algorithm (exhaustive trace exploration).

pub fn enumerate_traces(program: &str, max_states: usize) -> Result<Vec<(RVal, f64)>, String> {
    let mut stack_machines = vec![initial_machine(program)?];

    let mut finished = Vec::new();

    let mut visited = 0;

    while let Some(m) = stack_machines.pop() {
        visited += 1;

        if visited > max_states {
            return Err(format!("Out of limit: {}", max_states));
        }

        match resume(m)? {
            Msg::Done(value, m_done) => {
                finished.push((value, m_done.log_w));
            }

            Msg::Observe(_addr, dist, y_obs, mut m_obs) => {
                m_obs.log_w += dist.log_prob(&y_obs);

                send(&mut m_obs, y_obs);

                stack_machines.push(m_obs);
            }

            Msg::Sample(_addr, dist, m_sam) => {
                // Extract the support using finite_support

                let support = dist.finite_support()?;

                for (x, lp) in support {
                    let mut child = m_sam.fork();

                    child.log_w += lp;

                    send(&mut child, x);

                    stack_machines.push(child);
                }
            }
        }
    }

    Ok(finished)
}

// Groups runs that returned the same value and normalizes their probabilities.
pub fn posterior_table(runs: &[(RVal, f64)]) -> (Vec<(RVal, f64, f64)>, f64) {
    let mut log_masses: HashMap<RVal, f64> = HashMap::new();

    // Efficient O(N) aggregation
    for (val, lw) in runs {
        let entry = log_masses.entry(val.clone()).or_insert(f64::NEG_INFINITY);
        *entry = log_add_exp(*entry, *lw);
    }

    // Z = logsumexp of all the masses
    let log_z = log_masses
        .values()
        .cloned()
        .fold(f64::NEG_INFINITY, log_add_exp);

    // Convert the HashMap into a Vec to return it sorted/iterable
    // The tuple is (Value, NormalizedProbability, LogMass)
    let pmf: Vec<(RVal, f64, f64)> = log_masses
        .into_iter()
        .map(|(v, lw)| (v, (lw - log_z).exp(), lw))
        .collect();

    (pmf, log_z)
}