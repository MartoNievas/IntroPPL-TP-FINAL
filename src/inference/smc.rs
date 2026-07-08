/*

Modulo que implemente el algoritmo de inferencia Secuancial Monte Carlo, Aqui es donde brilla el objeto Machine ya que mantenemos una poblacion de N maquinas (Vec<Machine>)

*/

use crate::interpreter::{initial_machine, resume, send, Machine, Msg};
use crate::parser::value::RVal;
use rand::prelude::*;
use crate::parser::sexpr::Form;
use crate::parser::sexpr::parse;

/// Ejecuta el algoritmo Sequential Monte Carlo con N partículas.
pub fn run_smc<R: Rng + ?Sized>(
    program: &str,
    n_particles: usize,
    rng: &mut R,
) -> Result<Vec<RVal>, String> {
    // Aqui hacemos las verificaciones estaticas del AST
    let forms = parse(program)?;

    // Verificamos las formas
    check_scm_safety(&forms)?;


    // Parseamos el AST una sola vez y lo inicializamos en la máquina base
    let base_m = initial_machine(program)?;

    // 1. Inicializamos las N partículas usando la clonación ultrarrápida de memoria
    let mut particles: Vec<Machine> = Vec::with_capacity(n_particles);
    for _ in 0..n_particles {
        particles.push(base_m.fork());
    }

    loop {
        // 2. Avanzar todas las partículas hasta su próximo punto de sincronización
        let mut messages = Vec::with_capacity(n_particles);
        
        
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
                    
                    let lp = dist.log_prob(&y_obs);

                    m.log_w += lp;
                    log_increments.push(lp);

                    // Inyectamos el valor observado para que la máquina pueda continuar
                    send(&mut m, y_obs);
                    paused_machines.push(m);
                }
                // Deteccion de desincronización dinamico en tiempo de ejecucion
                _ => return Err("SMC Desynchronization Error: Particles reached divergent execution states. All particles in Sequential Monte Carlo must encounter the exact same sequence of 'observe' statements.".into()),
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

// Funcion auxiliar para el analisis estatico del AST para detectar desincronizacion en el algortimo SCM
fn check_scm_safety(forms: &[Form]) -> Result<(), String> {

    for form in forms {
        check_form(form)?;
    }
    Ok(())
}

// Funcion recursiva que retorna true si la forma contiene al menos un `observe`
// Falla si encuentra un `observe` en un lugar estructuralmente peligroso
fn check_form(form: &Form) -> Result<bool, String> {
    match form {
        Form::Int(_) | Form::Float(_) | Form::Bool(_) | Form::Str(_) | Form::Nil | Form::Symbol(_) => {
            Ok(false)
        }
        
        Form::List(list, _list_type) => {
            if list.is_empty() {
                return Ok(false);
            }

            if let Form::Symbol(head) = &list[0] {
                match head.as_str() {
                    "observe" => {
                        // Verificamos los argumentos por si tienen observes anidados
                        for arg in &list[1..] {
                            check_form(arg)?;
                        }
                        Ok(true) // Notificamos hacia arriba que encontramos un observe
                    }
                    
                    "if" => {
                        if list.len() == 4 {
                            let _ = check_form(&list[1])?; // Test
                            
                            let then_has_obs = check_form(&list[2])?;
                            let else_has_obs = check_form(&list[3])?;
                            
                            
                            if then_has_obs || else_has_obs {
                                return Err(
                                    "SMC Static Analysis Error: Found an 'observe' statement inside an 'if' branch. \
                                     SMC requires a deterministic observation flow. Please move the observation outside the conditional.".into()
                                );
                            }
                            return Ok(false);
                        }
                        Ok(false)
                    }
                    
                    "fn" | "defn" => {
                        let start_idx = if head.as_str() == "defn" { 3 } else { 2 };
                        
                        if list.len() > start_idx {
                            for expr in &list[start_idx..] {
                                let has_obs = check_form(expr)?;
                                
                                
                                if has_obs {
                                    return Err(
                                        "SMC Static Analysis Error: Found an 'observe' statement inside a 'fn' definition. \
                                         Functions can be called dynamically, which breaks SMC synchronization guarantees.".into()
                                    );
                                }
                            }
                        }
                        Ok(false)
                    }
                    
                    "let" => {
                        if list.len() >= 3 {
                            if let Form::List(binds, _list_type) = &list[1] {
                                let mut has_obs = false;
                                // Revisamos las expresiones asignadas a las variables
                                for i in (1..binds.len()).step_by(2) {
                                    has_obs |= check_form(&binds[i])?;
                                }
                                // Revisamos el cuerpo del let
                                for expr in &list[2..] {
                                    has_obs |= check_form(expr)?;
                                }
                                return Ok(has_obs);
                            }
                        }
                        Ok(false)
                    }
                    
                    _ => {
                        // Llamada estándar. Verificamos sus argumentos.
                        let mut has_obs = false;
                        for arg in list {
                            has_obs |= check_form(arg)?;
                        }
                        Ok(has_obs)
                    }
                }
            } else {
                // Si el primer elemento no es un símbolo, revisamos toda la lista
                let mut has_obs = false;
                for arg in list {
                    has_obs |= check_form(arg)?;
                }
                Ok(has_obs)
            }
        }
    }
}



// Función auxiliar para avanzar hasta el próximo 'Observe' o hasta que termine el programa.
// Los samples intermedios se resuelven automáticamente muestreando el prior.
fn advance_until_sync<R: Rng + ?Sized>(mut m: Machine, rng: &mut R) -> Result<Msg, String> {
    loop {
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

// Función auxiliar para re-muestreo: selecciona un índice según sus probabilidades categóricas.
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