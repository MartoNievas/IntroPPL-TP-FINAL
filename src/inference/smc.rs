/*

Modulo que implemente el algoritmo de inferencia Secuancial Monte Carlo, Aqui es donde brilla el objeto Machine ya que mantenemos una poblacion de N maquinas (Vec<Machine>)


*/

use crate::interpreter::{initial_machine, resume, send, Machine, Msg};
use crate::parser::value::RVal;
use rand::prelude::*;

// Función auxiliar para avanzar hasta el próximo 'Observe' o hasta que termine el programa.
// Los samples intermedios se resuelven automáticamente muestreando el prior.
fn advance_until_sync<R: Rng + ?Sized>(mut m: Machine, rng: &mut R) -> Result<Msg, String> {
    loop {
        // OPTIMIZACIÓN: Pasamos 'm' por valor sin .clone() porque ya somos dueños de ella
        match resume(m)? {
            Msg::Sample(_addr, dist, mut next_m) => {
                // Muestreamos de la distribución prior
                let sample_val = dist.sample(rng);
                send(&mut next_m, sample_val);
                m = next_m;
            }
            // Al encontrarnos un Observe o Done, devolvemos el mensaje al controlador 
            other => return Ok(other),
        }
    }
}

/// Ejecuta el algoritmo Sequential Monte Carlo con N partículas.
pub fn run_smc<R: Rng + ?Sized>(
    program: &str,
    n_particles: usize,
    rng: &mut R,
) -> Result<Vec<RVal>, String> {
    // 1. Inicializamos las N partículas todas iguales
    let mut particles: Vec<Machine> = Vec::with_capacity(n_particles);
    for _ in 0..n_particles {
        particles.push(initial_machine(program)?);
    }

    loop {
        // 2. Avanzar todas las partículas hasta su próximo punto de sincronización
        let mut messages = Vec::with_capacity(n_particles);
        
        // OPTIMIZACIÓN: Usamos .into_iter() para mover las máquinas en lugar de clonarlas
        for p in particles.into_iter() {
            messages.push(advance_until_sync(p, rng)?);
        }

        // 3. Si todas las partículas terminaron el programa, devolvemos los resultados
        if messages.iter().all(|msg| matches!(msg, Msg::Done(_, _))) {
            return Ok(messages
                .into_iter()
                .map(|msg| if let Msg::Done(val, _) = msg { val } else { unreachable!() })
                .collect());
        }

        // 4. Procesar el paso de observación
        let mut log_increments = Vec::with_capacity(n_particles);
        let mut paused_machines = Vec::with_capacity(n_particles);

        for msg in messages {
            match msg {
                Msg::Observe(_addr, dist, y_obs, mut m) => {
                    let obs_val = RVal::Float(y_obs);
                    let lp = dist.log_prob(&obs_val);

                    m.log_w += lp;
                    log_increments.push(lp);

                    // Inyectamos el valor observado para que la máquina pueda continuar
                    send(&mut m, obs_val);
                    paused_machines.push(m);
                }
                _ => return Err("Desincronización en SMC: las partículas llegaron a puntos de control distintos".into()),
            }
        }

        // 5. Normalización Softmax numéricamente estable
        let max_lp = log_increments
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let weights: Vec<f64> = log_increments
            .iter()
            .map(|&w| (w - max_lp).exp())
            .collect();

        let sum_w: f64 = weights.iter().sum();
        let probs: Vec<f64> = weights.iter().map(|w| w / sum_w).collect();

        // 6. Re-muestreo multinomial
        let mut new_particles = Vec::with_capacity(n_particles);
        for _ in 0..n_particles {
            let parent_idx = sample_categorical(&probs, rng);
            // Aquí sí es legítimo e indispensable usar .fork() para duplicar las ganadoras
            new_particles.push(paused_machines[parent_idx].fork());
        }
        particles = new_particles;
    }
}

/// Función auxiliar para re-muestreo: selecciona un índice según sus probabilidades categóricas.
fn sample_categorical<R: Rng + ?Sized>(probs: &[f64], rng: &mut R) -> usize {
    let u: f64 = rng.random();
    let mut cumsum = 0.0;
    for (i, &p) in probs.iter().enumerate() {
        cumsum += p;
        if u <= cumsum {
            return i;
        }
    }
    probs.len() - 1
}