mod parser;
mod interpreter;
mod inference;

use std::env;
use std::time::Instant;

use PPL_TP_FINAL::inference::lw::likelihood_weighting;
use PPL_TP_FINAL::inference::smc::run_smc;
use PPL_TP_FINAL::inference::ssmh::single_site_mh;
use PPL_TP_FINAL::inference::bbvi::run_bbvi;
use PPL_TP_FINAL::inference::exact_enumeration::{enumerate_traces, posterior_table};
use PPL_TP_FINAL::parser::value::RVal;
use rand::rngs::StdRng;
use rand::SeedableRng;

// ── Colores ANSI para que la salida se lea mejor en una presentación en vivo ──
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";

/// Extrae el valor f64 de un RVal numérico.
fn as_f64(val: &RVal) -> f64 {
    match val {
        RVal::Float(f) => *f,
        RVal::Int(i) => *i as f64,
        _ => panic!("Se esperaba un valor numerico, se obtuvo: {val:?}"),
    }
}

/// Diagnósticos básicos de una cadena MCMC: media, desvío estándar,
/// autocorrelación lag-1 y tamaño de muestra efectivo (ESS) aproximado.
/// El ESS asume decaimiento geométrico de la autocorrelación, que es una
/// aproximación razonable para cadenas simples como Single-Site MH.
struct ChainDiagnostics {
    mean: f64,
    std_dev: f64,
    lag1_autocorr: f64,
    ess: f64,
}

fn chain_diagnostics(chain: &[f64]) -> ChainDiagnostics {
    let n = chain.len();
    let mean = chain.iter().sum::<f64>() / n as f64;
    let variance = chain.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    let lag1_autocorr = if n > 1 && variance > 0.0 {
        let cov: f64 = chain
            .windows(2)
            .map(|w| (w[0] - mean) * (w[1] - mean))
            .sum::<f64>()
            / (n - 1) as f64;
        cov / variance
    } else {
        0.0
    };

    // ESS ≈ n * (1 - rho) / (1 + rho)  (rho = autocorrelacion lag-1)
    let rho = lag1_autocorr.clamp(-0.999, 0.999);
    let ess = n as f64 * (1.0 - rho) / (1.0 + rho);

    ChainDiagnostics { mean, std_dev, lag1_autocorr, ess }
}

fn print_header(title: &str) {
    println!("\n{BOLD}{CYAN}{:=^80}{RESET}", format!(" {title} "));
}

fn print_ok(msg: &str) {
    println!("   {GREEN}[OK]{RESET} {msg}");
}

fn print_err(msg: &str) {
    println!("   {RED}[ERROR]{RESET} {msg}");
}

/// Imprime encabezado + modelo fuente, corre el bloque de la demo,
/// y reporta cuánto tardó. No hace panic si `f` maneja sus propios
/// errores internamente (ver demos abajo).
fn run_demo<F: FnOnce()>(title: &str, model: &str, f: F) {
    print_header(title);
    println!("Modelo ejecutado:\n{}", model.trim());
    println!();
    let start = Instant::now();
    f();
    println!("   (tiempo: {:.2?})", start.elapsed());
}

struct Demo {
    id: usize,
    label: &'static str,
    run: fn(&mut StdRng),
}

// ============================================================================
// DEMOSTRACION 1: Likelihood Weighting
// ============================================================================
fn demo_lw(rng: &mut StdRng) {
    let model = r#"
        ; Prior: mu ~ Normal(0, 1)
        ; Likelihood: Observamos el valor 3.0 desde Normal(mu, 1)
        (let [mu (sample (normal 0 1))]
            (observe (normal mu 1) 3.0)
            mu)
    "#;

    run_demo("1. LIKELIHOOD WEIGHTING (LW)", model, || {
        let n_particles = 5000;
        println!("Ejecutando LW con {n_particles} particulas...");

        match likelihood_weighting(model, n_particles, rng) {
            Ok((vals, weights)) => {
                let mean: f64 = vals
                    .iter()
                    .zip(weights.iter())
                    .map(|(v, w)| as_f64(v) * w)
                    .sum();
                print_ok(&format!(
                    "Media a posteriori estimada: {mean:.4} (Analitica: ~1.5000)"
                ));
            }
            Err(e) => print_err(&format!("Fallo en Likelihood Weighting: {e}")),
        }
    });
}

// ============================================================================
// DEMOSTRACION 2: Sequential Monte Carlo (SMC)
// ============================================================================
fn demo_smc(rng: &mut StdRng) {
    let model = r#"
        ; Prior: p ~ Beta(2, 2)
        ; Likelihood: Observamos 3 caras (true) y 1 cruz (false)
        (let [p (sample (beta 2.0 2.0))]
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) false)
            p)
    "#;

    run_demo("2. SEQUENTIAL MONTE CARLO (SMC / Filtro de Particulas)", model, || {
        let n_particles = 2000;
        println!("Ejecutando SMC con {n_particles} particulas sincronizadas...");

        match run_smc(model, n_particles, rng) {
            Ok(vals) => {
                let mean: f64 = vals.iter().map(as_f64).sum::<f64>() / (n_particles as f64);
                print_ok(&format!(
                    "Probabilidad 'p' estimada: {mean:.4} (Analitica: ~0.6250)"
                ));
            }
            Err(e) => print_err(&format!("Fallo en SMC: {e}")),
        }
    });
}

// ============================================================================
// DEMOSTRACION 3: Analisis Estatico de SMC (Proteccion contra desincronizacion)
// ============================================================================
fn demo_smc_safety(rng: &mut StdRng) {
    let model = r#"
        ; Este programa rompe SMC porque el 'observe' esta oculto en una rama
        ; estocastica, lo que desincroniza a las particulas en el tiempo de ejecucion.
        (if (sample (bernoulli 0.5))
            (observe (normal 0 1) 1.0)
            0)
    "#;

    run_demo("3. SEGURIDAD SMC: ANALISIS ESTATICO DE TRAZAS", model, || {
        println!("Analizando AST y previniendo ejecucion desincronizada...");

        match run_smc(model, 100, rng) {
            Ok(_) => print_err("Fallo de seguridad: el modelo debio ser rechazado por el linter."),
            Err(e) => {
                print_ok("Analisis Estatico exitoso. Ejecucion abortada antes de instanciar particulas.");
                println!("   Detalle del Linter:\n     >> {e}");
            }
        }
    });
}

// ============================================================================
// DEMOSTRACION 4: Single-Site Metropolis-Hastings (MCMC)
// ============================================================================
fn demo_ssmh(rng: &mut StdRng) {
    let model = r#"
        ; Prior para pendiente (m) y sesgo (b)
        (let [m (sample (normal 0 2))
              b (sample (normal 0 2))]
            ; Observamos puntos ruidosos de la recta y = 2x + 1
            (observe (normal (+ (* m 1) b) 0.5) 3.0)
            (observe (normal (+ (* m 2) b) 0.5) 5.0)
            m)
    "#;

    run_demo("4. SINGLE-SITE METROPOLIS-HASTINGS (MCMC)", model, || {
        let steps = 4000;
        let warmup = 1000;
        println!("Ejecutando SSMH (Pasos: {steps}, Warmup: {warmup})...");

        match single_site_mh(model, rng, steps, warmup) {
            Ok(chain) => {
                let mean: f64 = chain.iter().map(as_f64).sum::<f64>() / (steps as f64);
                print_ok(&format!(
                    "Pendiente 'm' estimada: {mean:.4} (Esperada: ~2.0000)"
                ));
            }
            Err(e) => print_err(&format!("Fallo en SSMH: {e}")),
        }
    });
}

// ============================================================================
// DEMOSTRACION 5: Black-Box Variational Inference (BBVI)
// ============================================================================
fn demo_bbvi(rng: &mut StdRng) {
    let model = r#"
        ; Prior: mu ~ Normal(0.0, 5.0) (incertidumbre amplia)
        ; Likelihood: Observamos 3 mediciones que apuntan a una media de ~2.2
        (let [mu (sample (normal 0.0 5.0))]
          (observe (normal mu 1.0) 2.0)
          (observe (normal mu 1.0) 2.5)
          (observe (normal mu 1.0) 2.1)
          mu)
    "#;

    run_demo("5. BLACK-BOX VARIATIONAL INFERENCE (BBVI)", model, || {
        let n_samples: usize = 20;
        let steps: usize = 250;
        let lr = 0.05;

        println!("Optimizando ELBO con Adam (Pasos: {steps}, Muestras/Lote: {n_samples}, LR: {lr})...");

        match run_bbvi(model, steps, n_samples, lr, rng) {
            Ok((elbo_history, theta_opt)) => {
                let elbo_inicial = elbo_history.first().unwrap();
                let elbo_final = elbo_history.last().unwrap();

                println!("\nResultados de la Optimizacion Variacional:");
                println!("   ELBO Inicial : {elbo_inicial:.4}");
                println!("   ELBO Final   : {elbo_final:.4}");

                if elbo_final > elbo_inicial {
                    print_ok("La ELBO ascendio con exito. El optimizador redujo la divergencia.");
                } else {
                    println!("   [ADVERTENCIA] La ELBO no subio significativamente.");
                }

                println!("\n   Parametros Variacionales Optimizados (Theta):");
                for (addr, params) in theta_opt {
                    let fmt_addr = addr.join("/");
                    println!("      Direccion [{fmt_addr}]: {params:?}");
                }

                println!("\n   Nota: al observar 2.0, 2.5 y 2.1, Adam ajusta la media (mu) hacia ~2.2000.");
            }
            Err(e) => print_err(&format!("Fallo en BBVI: {e}")),
        }
    });
}

// ============================================================================
// DEMOSTRACION 6: Exact Enumeration
// ============================================================================
fn demo_enum(_rng: &mut StdRng) {
    let model = r#"
        ; Un padre tiene genotipo 0, 1 o 2 (copias de un alelo).
        ; Prior: genotipo ~ UniformeDiscreta(0, 3)
        ; Si el genotipo es 1, hay 50% de chance de pasar el gen.
        (let [genotipo (sample (uniform-discrete 0 3))
              p_expresion (if (= genotipo 0) 0.0
                           (if (= genotipo 1) 0.5 1.0))
              expresado (sample (bernoulli p_expresion))]
            (observe (bernoulli p_expresion) true)
            genotipo)
    "#;

    run_demo("6. EXACT ENUMERATION", model, || {
        println!("Explorando todos los estados posibles...");

        match enumerate_traces(model, 1000) {
            Ok(runs) => {
                let (pmf, log_z) = posterior_table(&runs);
                print_ok(&format!("Enumeracion completada. Log Evidence (Z): {log_z:.4}"));
                println!("   Distribucion posterior sobre el genotipo:");
                for (val, prob) in pmf {
                    println!("      Genotipo {val:?}: {prob:.4}");
                }
            }
            Err(e) => print_err(&format!("Fallo en Enumeracion Exacta: {e}")),
        }
    });
}

// ============================================================================
// MAIN
// ============================================================================
fn main() {
    println!("Iniciando Demostracion: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Autor: Martin Nievas Wilberger");

    let args: Vec<String> = env::args().collect();
    let selected: Option<usize> = args.get(1).and_then(|s| s.parse().ok());

    let demos: Vec<Demo> = vec![
        Demo { id: 1, label: "Likelihood Weighting", run: demo_lw },
        Demo { id: 2, label: "Sequential Monte Carlo", run: demo_smc },
        Demo { id: 3, label: "Seguridad SMC (analisis estatico)", run: demo_smc_safety },
        Demo { id: 4, label: "Single-Site MH", run: demo_ssmh },
        Demo { id: 5, label: "BBVI", run: demo_bbvi },
        Demo { id: 6, label: "Exact Enumeration", run: demo_enum },
    ];

    if let Some(n) = selected {
        if !demos.iter().any(|d| d.id == n) {
            eprintln!("Numero de demo invalido: {n}. Usa un valor entre 1 y {}.", demos.len());
            std::process::exit(1);
        }
    }

    let total_start = Instant::now();

    for demo in &demos {
        if let Some(n) = selected {
            if n != demo.id {
                continue;
            }
        }
        // Cada demo arranca con su propia seed derivada del id, en vez de
        // compartir un rng entre todas. Así la demo N da siempre el mismo
        // resultado sin importar si corrió sola o junto a las demás.
        let mut demo_rng = StdRng::seed_from_u64(42 + demo.id as u64);
        (demo.run)(&mut demo_rng);
    }

    print_header("FIN DE LA DEMOSTRACION - HOPPL EN RUST");
    println!("Tiempo total: {:.2?}", total_start.elapsed());
    if selected.is_none() {
        println!("Tip: ejecuta `cargo run -- <numero>` para correr una sola demo (1-6).");
        for demo in &demos {
            println!("   {} -> {}", demo.id, demo.label);
        }
    }
}