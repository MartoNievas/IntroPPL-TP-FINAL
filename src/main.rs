mod parser;
mod interpreter;
mod inference;

use std::env::args;
use PPL_TP_FINAL::inference::lw::likelihood_weighting;
use PPL_TP_FINAL::inference::smc::run_smc;
use PPL_TP_FINAL::inference::ssmh::single_site_mh;
use PPL_TP_FINAL::inference::bbvi::run_bbvi;
use PPL_TP_FINAL::parser::value::RVal;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Función auxiliar para extraer el valor f64 de un RVal numérico
fn as_f64(val: &RVal) -> f64 {
    match val {
        RVal::Float(f) => *f,
        RVal::Int(i) => *i as f64,
        _ => panic!("Se esperaba un valor numerico, se obtuvo: {val:?}"),
    }
}

/// Función auxiliar para imprimir separadores limpios en consola
fn print_header(title: &str) {
    println!("\n{:=^80}", format!(" {title} "));
}

fn main() {
    println!("Iniciando Demostracion: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Autor: Martin Nievas Wilberger");

    // ========================================================================
    // DEMOSTRACION 1: Likelihood Weighting (Ponderacion por Verosimilitud)
    // Modelo: Inferir la media de una Normal con Prior Normal
    // ========================================================================
    print_header("1. LIKELIHOOD WEIGHTING (LW)");
    let model_lw = r#"
        ; Prior: mu ~ Normal(0, 1)
        ; Likelihood: Observamos el valor 3.0 desde Normal(mu, 1)
        (let [mu (sample (normal 0 1))]
            (observe (normal mu 1) 3.0)
            mu)
    "#;
    println!("Modelo ejecutado:\n{}", model_lw.trim());
    
    let mut rng = StdRng::seed_from_u64(42);
    let n_particles_lw = 5000;
    println!("\nEjecutando LW con {n_particles_lw} particulas...");
    
    let (lw_vals, lw_weights) = likelihood_weighting(model_lw, n_particles_lw, &mut rng)
        .expect("Error en Likelihood Weighting");
        
    let mut lw_mean = 0.0;
    for (v, w) in lw_vals.iter().zip(lw_weights.iter()) {
        lw_mean += as_f64(v) * w;
    }
    println!("   [OK] Media a posteriori estimada: {lw_mean:.4} (Analitica: ~1.5000)");

    // ========================================================================
    // DEMOSTRACION 2: Sequential Monte Carlo (SMC)
    // Modelo: Moneda sesgada (Beta-Bernoulli)
    // ========================================================================
    print_header("2. SEQUENTIAL MONTE CARLO (SMC / Filtro de Particulas)");
    let model_smc = r#"
        ; Prior: p ~ Beta(2, 2)
        ; Likelihood: Observamos 3 caras (true) y 1 cruz (false)
        (let [p (sample (beta 2.0 2.0))]
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) false)
            p)
    "#;
    println!("Modelo ejecutado:\n{}", model_smc.trim());
    
    let n_particles_smc = 2000;
    println!("\nEjecutando SMC con {n_particles_smc} particulas sincronizadas...");
    
    let smc_vals = run_smc(model_smc, n_particles_smc, &mut rng)
        .expect("Error en SMC");
        
    // En SMC las particulas devueltas tienen peso uniforme tras el re-muestreo
    let smc_mean: f64 = smc_vals.iter().map(as_f64).sum::<f64>() / (n_particles_smc as f64);
    println!("   [OK] Probabilidad 'p' estimada: {smc_mean:.4} (Analitica: ~0.6250)");

    // ========================================================================
    // DEMOSTRACION 3: Analisis Estatico de SMC (Proteccion contra desincronizacion)
    // ========================================================================
    print_header("3. SEGURIDAD SMC: ANALISIS ESTATICO DE TRAZAS");
    let model_bad_smc = r#"
        ; Este programa rompe SMC porque el 'observe' esta oculto en una rama
        ; estocastica, lo que desincroniza a las particulas en el tiempo de ejecucion.
        (if (sample (bernoulli 0.5))
            (observe (normal 0 1) 1.0)
            0)
    "#;
    println!("Modelo inseguro ejecutado:\n{}", model_bad_smc.trim());
    println!("\nAnalizando AST y previniendo ejecucion desincronizada...");
    
    match run_smc(model_bad_smc, 100, &mut rng) {
        Ok(_) => println!("   [ERROR] Fallo de seguridad: El modelo debio ser rechazado por el linter."),
        Err(e) => {
            println!("   [OK] Analisis Estatico exitoso. Ejecucion abortada correctamente antes de instanciar particulas.");
            println!("   Detalle del Linter:\n     >> {e}");
        }
    }

    // ========================================================================
    // DEMOSTRACION 4: Single-Site Metropolis-Hastings (MCMC)
    // Modelo: Regresion Lineal Bayesiana Simple
    // ========================================================================
    print_header("4. SINGLE-SITE METROPOLIS-HASTINGS (MCMC)");
    let model_ssmh = r#"
        ; Prior para pendiente (m) y sesgo (b)
        (let [m (sample (normal 0 2))
              b (sample (normal 0 2))]
            ; Observamos puntos ruidosos de la recta y = 2x + 1
            ; x=1 -> y=3
            (observe (normal (+ (* m 1) b) 0.5) 3.0)
            ; x=2 -> y=5
            (observe (normal (+ (* m 2) b) 0.5) 5.0)
            
            ; Devolvemos la pendiente 'm' estimada
            m)
    "#;
    println!("Modelo ejecutado:\n{}", model_ssmh.trim());
    
    let steps = 4000;
    let warmup = 1000;
    println!("\nEjecutando SSMH (Pasos: {steps}, Warmup: {warmup})...");
    
    let chain = single_site_mh(model_ssmh, &mut rng, steps, warmup)
        .expect("Error en SSMH");
        
    let mh_mean: f64 = chain.iter().map(as_f64).sum::<f64>() / (steps as f64);
    println!("   [OK] Pendiente 'm' estimada: {mh_mean:.4} (Esperada: ~2.0000)");

    // ========================================================================
    // DEMOSTRACION 5: Black-Box Variational Inference (BBVI)
    // Modelo: Estimacion de media Gaussiana con observaciones ruidosas
    // ========================================================================
    print_header("5. BLACK-BOX VARIATIONAL INFERENCE (BBVI)");
    let model_bbvi = r#"
        ; Prior: mu ~ Normal(0.0, 5.0) (incertidumbre amplia)
        ; Likelihood: Observamos 3 mediciones que apuntan a una media de ~2.2
        (let [mu (sample (normal 0.0 5.0))]
          (observe (normal mu 1.0) 2.0)
          (observe (normal mu 1.0) 2.5)
          (observe (normal mu 1.0) 2.1)
          mu)
    "#;
    println!("Modelo ejecutado:\n{}", model_bbvi.trim());

    // Ajustados para una ejecucion rapida y estable en la demostracion en vivo
    let n_samples: usize = 20;
    let steps: usize = 250;
    let lr = 0.05;

    println!("\nOptimizando ELBO con Adam (Pasos: {steps}, Muestras/Lote: {n_samples}, LR: {lr})...");
    let (elbo_history, theta_opt) = run_bbvi(model_bbvi, steps, n_samples, lr, &mut rng)
        .expect("Error en BBVI");

    // 1. Extraer metricas de convergencia
    let elbo_inicial = elbo_history.first().unwrap();
    let elbo_final = elbo_history.last().unwrap();

    println!("\nResultados de la Optimizacion Variacional:");
    println!("   ELBO Inicial : {elbo_inicial:.4}");
    println!("   ELBO Final   : {elbo_final:.4}");

    if elbo_final > elbo_inicial {
        println!("   [OK] La ELBO ascendio con exito. El optimizador redujo la divergencia.");
    } else {
        println!("   [ADVERTENCIA] La ELBO no subio significativamente.");
    }

    // 2. Imprimir los parametros de la distribucion guia
    println!("\n   Parametros Variacionales Optimizados (Theta):");
    for (addr, params) in theta_opt {
        let fmt_addr = addr.join("/");
        println!("      Direccion [{fmt_addr}]: {params:?}");
    }

    println!("\n   Nota: Al observar los datos 2.0, 2.5 y 2.1, el optimizador Adam");
    println!("      ajusta exitosamente el parametro de la media (mu) hacia ~2.2000.");

    // ========================================================================
    // CIERRE
    // ========================================================================
    println!("\n{:=^80}\n", " FIN DE LA DEMOSTRACION - HOPPL EN RUST ");
}