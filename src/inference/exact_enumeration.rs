/*
Módulo que implementa el algoritmo de inferencia Exact Enumeration (Enumeración Exacta).
Este algoritmo explora exhaustivamente todas las ramificaciones posibles de un programa
probabilístico con variables discretas finitas.
*/

use crate::interpreter::{initial_machine, resume, send, Msg, Machine};
use crate::parser::distribution::Distribution;
use crate::parser::value::RVal;

/// Función auxiliar matemática equivalente a `scipy.special.logsumexp` de Python.
/// Calcula log(exp(a) + exp(b)) de forma numéricamente estable.
fn log_add_exp(a: f64, b: f64) -> f64 {
    let m = a.max(b);
    if m == f64::NEG_INFINITY {
        m
    } else {
        m + ((a - m).exp() + (b - m).exp()).ln()
    }
}


/// Ejecuta el algoritmo de Enumeración Exacta (exploración exhaustiva de trazas).

pub fn enumerate_traces(program: &str, max_states: usize) -> Result<Vec<(RVal, f64)>, String> {
    
    let mut stack_machines = vec![initial_machine(program)?];
    let mut finished  = Vec::new();
    let mut visited = 0;

    while let Some(m) = stack_machines.pop() {
        visited += 1;
        if visited > max_states {
            return Err(format!("Out of limit: {}",max_states));
        }

        match resume(m)? {
            Msg::Done(value, m_done) => {
                finished.push((value, m_done.log_w));
            }

            Msg::Observe(_addr, dist, y_obs,mut m_obs) => {
                m_obs.log_w += dist.log_prob(&y_obs);

                send(&mut m_obs, y_obs);
                stack_machines.push(m_obs);
            }

            Msg::Sample(_addr, dist, m_sam) => {
                // Extraemos el soporte usando finite_support
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


// Agrupa las ejecuciones que retornaron el mismo valor y normaliza sus probabilidades.
pub fn posterior_table(runs: &[(RVal, f64)]) -> (Vec<(RVal, f64)>, f64) {
    let mut unique_vals: Vec<RVal> = Vec::new();
    let mut log_masses: Vec<f64> = Vec::new();

    // Agregación de ejecuciones con el mismo valor de retorno (equivalente al for value, lw in runs)
    for (val, lw) in runs {
        if let Some(idx) = unique_vals.iter().position(|v| v == val) {
            log_masses[idx] = log_add_exp(log_masses[idx], *lw); // np.logaddexp
        } else {
            unique_vals.push(val.clone());
            log_masses.push(*lw);
        }
    }

    // Z = logsumexp([log_mass[k] for k in keys])
    let log_z = log_masses
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, log_add_exp);

    // Calculamos prob = math.exp(log_mass[k] - Z)
    let pmf: Vec<(RVal, f64)> = unique_vals
        .into_iter()
        .zip(log_masses.into_iter())
        .map(|(v, lw)| (v, (lw - log_z).exp()))
        .collect();

    (pmf, log_z)
}