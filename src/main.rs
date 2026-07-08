/*

Módulo de entrada del programa, para ejecución de una demostración o ejecución de archivos de codigo del hoppl.
Con los siguientes modos de uso:

    - Modo demostracion: cargo run -> corre la demo entera | cargo run -- <num_demo> -> corre demo especifica.
    - Ejecución determinística: cargo run -- <archivo.hoppl> -> corre un programa sin 'sample'/'observe'.
    - Ejecución de código hoppl probabilístico: cargo run -- <archivo.hoppl> <algoritmo de inferencia>.

    numero de demos = 6
    algoritmos de inferencia:
        - lw
        - ssmh
        - smc
        - bbvi
        - exact numeration

*/

mod inference;
mod interpreter;
mod parser;

use std::env;
use std::fs;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;
use ppl_tp_final::inference::bbvi::run_bbvi;
use ppl_tp_final::inference::exact_enumeration::{enumerate_traces, posterior_table};
use ppl_tp_final::inference::lw::likelihood_weighting;
use ppl_tp_final::inference::smc::run_smc;
use ppl_tp_final::inference::ssmh::single_site_mh;
use crate::interpreter::{initial_machine, resume, Msg};
use ppl_tp_final::parser::value::RVal;
use term_table::{row::Row, table_cell::*, Table, TableStyle};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";

fn as_f64(val: &RVal) -> f64 {
    match val {
        RVal::Float(f) => *f,
        RVal::Int(i) => *i as f64,
        RVal::Bool(b) => if *b { 1.0 } else { 0.0 }, 
        _ => panic!("Se esperaba un valor numerico, se obtuvo: {val:?}"),
    }
}

fn pause() {
    print!("\n   Presiona ENTER para continuar...");
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
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

fn print_warn(msg: &str) {
    println!("   {YELLOW}[AVISO]{RESET} {msg}");
}

fn weighted_mean_var(vals: &[RVal], weights: &[f64]) -> (f64, f64) {
    let mean: f64 = vals
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| as_f64(v) * w)
        .sum();
    let var: f64 = vals
        .iter()
        .zip(weights.iter())
        .map(|(v, w)| w * (as_f64(v) - mean).powi(2))
        .sum();
    (mean, var)
}

fn effective_sample_size(weights: &[f64]) -> f64 {
    let sum_sq: f64 = weights.iter().map(|w| w * w).sum();
    1.0 / sum_sq
}

fn sample_mean_std_err(vals: &[RVal]) -> (f64, f64) {
    let n = vals.len() as f64;
    let mean: f64 = vals.iter().map(as_f64).sum::<f64>() / n;
    let var: f64 = vals.iter().map(|x| (as_f64(x) - mean).powi(2)).sum::<f64>() / n;
    let std_err = (var / n).sqrt();
    (mean, std_err)
}

fn autocorrelations(xs: &[f64], mean: f64, var: f64, max_lag: usize) -> Vec<f64> {
    let n = xs.len();
    let mut rhos = Vec::with_capacity(max_lag + 1);
    for k in 0..=max_lag {
        let cov: f64 = (0..n - k).map(|i| (xs[i] - mean) * (xs[i + k] - mean)).sum::<f64>() / n as f64;
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

fn mcmc_mean_std_err_ess(chain: &[RVal]) -> (f64, f64, f64) {
    let xs: Vec<f64> = chain.iter().map(as_f64).collect();
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

fn fmt_log_mass(log_mass: f64) -> String {
    if log_mass == f64::NEG_INFINITY {
        "-∞".to_string()
    } else if log_mass.is_infinite() {
        "∞".to_string()
    } else {
        format!("{:.4}", log_mass)
    }
}

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
                let (mean, var) = weighted_mean_var(&vals, &weights);
                let std_err = (var / n_particles as f64).sqrt();
                let ess = effective_sample_size(&weights);
                print_ok(&format!(
                    "Media a posteriori estimada: {mean:.4} ± {std_err:.4} (Analitica: ~1.5000)"
                ));
                print_ok(&format!(
                    "Effective Sample Size (ESS): {ess:.1} / {n_particles}"
                ));
            }
            Err(e) => print_err(&format!("Fallo en Likelihood Weighting: {e}")),
        }
    });
}

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

    run_demo(
        "2. SEQUENTIAL MONTE CARLO (SMC / Filtro de Particulas)",
        model,
        || {
            let n_particles = 2000;
            println!("Ejecutando SMC con {n_particles} particulas sincronizadas...");

            match run_smc(model, n_particles, rng) {
                Ok(vals) => {
                    let (mean, std_err) = sample_mean_std_err(&vals);
                    print_ok(&format!(
                        "Probabilidad 'p' estimada: {mean:.4} ± {std_err:.4} (Analitica: ~0.6250)"
                    ));
                }
                Err(e) => print_err(&format!("Fallo en SMC: {e}")),
            }
        },
    );
}

fn demo_smc_safety(rng: &mut StdRng) {
    let model = r#"
        ; Este programa rompe SMC porque el 'observe' esta oculto en una rama
        ; estocastica, lo que desincroniza a las particulas en el tiempo de ejecucion.
        (if (sample (bernoulli 0.5))
            (observe (normal 0 1) 1.0)
            0)
    "#;

    run_demo(
        "3. SEGURIDAD SMC: ANALISIS ESTATICO DE TRAZAS",
        model,
        || {
            println!("Analizando AST y previniendo ejecucion desincronizada...");

            match run_smc(model, 100, rng) {
                Ok(_) => {
                    print_err("Fallo de seguridad: el modelo debio ser rechazado por el linter.")
                }
                Err(e) => {
                    print_ok("Analisis Estatico exitoso. Ejecucion abortada antes de instanciar particulas.");
                    println!("   Detalle del Linter:\n     >> {e}");
                }
            }
        },
    );
}

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
                let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);

                print_ok(&format!(
                    "Pendiente 'm' estimada: {mean:.4} ± {std_err:.4} (Esperada: 2.0)"
                ));
                print_ok(&format!(
                    "Effective Sample Size (ESS, por autocorrelacion): {ess:.1} / {steps}"
                ));
            }
            Err(e) => print_err(&format!("Fallo en SSMH: {e}")),
        }
    });
}

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

        println!(
            "Optimizando ELBO con Adam (Pasos: {steps}, Muestras/Lote: {n_samples}, LR: {lr})..."
        );

        match run_bbvi(model, steps, n_samples, lr, rng) {
            Ok((elbo_history, theta_opt)) => {
                let elbo_inicial = elbo_history.first().unwrap();
                let elbo_final = elbo_history.last().unwrap();
                let delta = elbo_final - elbo_inicial;

                println!("\nResultados de la Optimizacion Variacional:");
                println!("   ELBO Inicial : {elbo_inicial:.4}");
                println!("   ELBO Final   : {elbo_final:.4}");
                println!("   Delta ELBO   : {delta:+.4}");

                if delta > 0.0 {
                    print_ok("La ELBO ascendio con exito. El optimizador redujo la divergencia.");
                } else {
                    print_warn("La ELBO no subio significativamente.");
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
                let (mut pmf, log_z) = posterior_table(&runs);
                print_ok(&format!(
                    "Enumeracion completada. Log Evidence (Z): {log_z:.4}"
                ));

                pmf.sort_by(|a, b| a.0.as_i64().cmp(&b.0.as_i64()));

                print_ok(&format!("Estados explorados totales: {}", runs.len()));

                let mut table = Table::builder().style(TableStyle::elegant()).build();

                table.add_row(Row::new(vec![
                    TableCell::builder("Valor")
                        .alignment(Alignment::Center)
                        .build(),
                    TableCell::builder("Prob")
                        .alignment(Alignment::Center)
                        .build(),
                    TableCell::builder("Log mass")
                        .alignment(Alignment::Right)
                        .build(),
                ]));

                for (value, prob, log_mass) in pmf {
                    let prob_str = if prob < 0.0001 && prob > 0.0 {
                        format!("{:.4e}", prob)
                    } else {
                        format!("{:.4}", prob)
                    };

                    table.add_row(Row::new(vec![
                        TableCell::builder(value.to_string())
                            .alignment(Alignment::Center)
                            .build(),
                        TableCell::builder(prob_str)
                            .alignment(Alignment::Center)
                            .build(),
                        TableCell::builder(fmt_log_mass(log_mass))
                            .alignment(Alignment::Right)
                            .build(),
                    ]));
                }

                let table_str = table.render();
                for line in table_str.lines() {
                    println!("  {}", line);
                }
            }
            Err(e) => print_err(&format!("Fallo en Enumeracion Exacta: {e}")),
        }
    });
}

#[derive(Debug, Clone, Copy)]
enum Algorithm {
    Lw,
    Ssmh,
    Smc,
    Bbvi,
    Enumeration,
}

impl Algorithm {
    fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().replace('_', "-").as_str() {
            "lw" => Some(Algorithm::Lw),
            "ssmh" => Some(Algorithm::Ssmh),
            "smc" => Some(Algorithm::Smc),
            "bbvi" => Some(Algorithm::Bbvi),
            "exact-enumeration" | "exact enumeration" | "enum" | "exact" | "enumeration" => {
                Some(Algorithm::Enumeration)
            }
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Algorithm::Lw => "Likelihood Weighting",
            Algorithm::Ssmh => "Single-Site Metropolis-Hastings",
            Algorithm::Smc => "Sequential Monte Carlo",
            Algorithm::Bbvi => "Black-Box Variational Inference",
            Algorithm::Enumeration => "Exact Enumeration",
        }
    }
}

fn load_model_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            print_err(&format!("No se pudo leer el archivo '{path}': {e}"));
            std::process::exit(1);
        }
    }
}

/// Corre un programa HOPPL que se asume **determinístico** (sin 'sample' ni
/// 'observe'): en vez de invocar un motor de inferencia, avanza la máquina
/// CEK directamente hasta que termine. Si el programa resulta tener efectos
/// probabilísticos, se detecta apenas aparece el primero y se informa con
/// un mensaje claro en vez de fallar de forma confusa.
fn run_deterministic_model(file_path: &str, model: &str) {
    print_header("MODO ARCHIVO: Ejecucion Deterministica");
    println!("Modelo cargado:\n{}", model.trim());
    println!();

    let start = Instant::now();

    let machine = match initial_machine(model) {
        Ok(m) => m,
        Err(e) => {
            print_err(&format!("Error al inicializar el programa: {e}"));
            println!("   (tiempo: {:.2?})", start.elapsed());
            return;
        }
    };

    match resume(machine) {
        Ok(Msg::Done(value, _)) => {
            let value_str = value.to_string();
            print_ok(&format!("Resultado: {value_str:?}"));
        }
        Ok(Msg::Sample(addr, _, _)) => {
            print_err(&format!(
                "El programa no es deterministico: se encontro un 'sample' en la direccion {addr:?}."
            ));
            println!("   Este modo es solo para programas sin 'sample'/'observe'. Corre con un algoritmo de inferencia en su lugar:");
            println!(
                "      cargo run -- {file_path} <algoritmo>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }
        Ok(Msg::Observe(addr, _, _, _)) => {
            print_err(&format!(
                "El programa no es deterministico: se encontro un 'observe' en la direccion {addr:?}."
            ));
            println!("   Este modo es solo para programas sin 'sample'/'observe'. Corre con un algoritmo de inferencia en su lugar:");
            println!(
                "      cargo run -- {file_path} <algoritmo>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }
        Err(e) => print_err(&format!("Error de ejecucion: {e}")),
    }

    println!("   (tiempo: {:.2?})", start.elapsed());
}

fn run_algorithm_on_model(algorithm: Algorithm, model: &str, rng: &mut StdRng) {
    const N_PARTICLES_LW: usize = 5000;
    const N_PARTICLES_SMC: usize = 2000;
    const SSMH_STEPS: usize = 4000;
    const SSMH_WARMUP: usize = 1000;
    const BBVI_STEPS: usize = 250;
    const BBVI_SAMPLES: usize = 20;
    const BBVI_LR: f64 = 0.05;
    const ENUM_MAX_TRACES: usize = 100000;

    print_header(&format!("MODO ARCHIVO: {}", algorithm.label()));
    println!("Modelo cargado:\n{}", model.trim());
    println!();

    let start = Instant::now();

    match algorithm {
        Algorithm::Lw => {
            println!("  Ejecutando LW con {N_PARTICLES_LW} particulas...");
            match likelihood_weighting(model, N_PARTICLES_LW, rng) {
                Ok((vals, weights)) => {
                    let (mean, var) = weighted_mean_var(&vals, &weights);
                    let std_err = (var / N_PARTICLES_LW as f64).sqrt();
                    let ess = effective_sample_size(&weights);
                    print_ok(&format!("Media a posteriori estimada: {mean:.4} ± {std_err:.4}"));
                    print_ok(&format!(
                        "Effective Sample Size (ESS): {ess:.1} / {N_PARTICLES_LW}"
                    ));
                }
                Err(e) => print_err(&format!("Fallo en Likelihood Weighting: {e}")),
            }
        }
        Algorithm::Smc => {
            println!("Ejecutando SMC con {N_PARTICLES_SMC} particulas sincronizadas...");
            match run_smc(model, N_PARTICLES_SMC, rng) {
                Ok(vals) => {
                    let (mean, std_err) = sample_mean_std_err(&vals);
                    print_ok(&format!("Valor esperado estimado: {mean:.4} ± {std_err:.4}"));
                }
                Err(e) => print_err(&format!("Fallo en SMC: {e}")),
            }
        }
        Algorithm::Ssmh => {
            println!("Ejecutando SSMH (Pasos: {SSMH_STEPS}, Warmup: {SSMH_WARMUP})...");
            match single_site_mh(model, rng, SSMH_STEPS, SSMH_WARMUP) {
                Ok(chain) => {
                    let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);
                    print_ok(&format!("Valor estimado: {mean:.4} ± {std_err:.4}"));
                    print_ok(&format!(
                        "Effective Sample Size (ESS, por autocorrelacion): {ess:.1} / {SSMH_STEPS}"
                    ));
                }
                Err(e) => print_err(&format!("Fallo en SSMH: {e}")),
            }
        }
        Algorithm::Bbvi => {
            println!(
                "Optimizando ELBO con Adam (Pasos: {BBVI_STEPS}, Muestras/Lote: {BBVI_SAMPLES}, LR: {BBVI_LR})..."
            );
            match run_bbvi(model, BBVI_STEPS, BBVI_SAMPLES, BBVI_LR, rng) {
                Ok((elbo_history, theta_opt)) => {
                    let elbo_inicial = elbo_history.first().unwrap();
                    let elbo_final = elbo_history.last().unwrap();
                    let delta = elbo_final - elbo_inicial;

                    println!("\nResultados de la Optimizacion Variacional:");
                    println!("   ELBO Inicial : {elbo_inicial:.4}");
                    println!("   ELBO Final   : {elbo_final:.4}");
                    println!("   Delta ELBO   : {delta:+.4}");

                    if delta > 0.0 {
                        print_ok(
                            "La ELBO ascendio con exito. El optimizador redujo la divergencia.",
                        );
                    } else {
                        print_warn("La ELBO no subio significativamente.");
                    }

                    println!("\n   Parametros Variacionales Optimizados (Theta):");
                    for (addr, params) in theta_opt {
                        let fmt_addr = addr.join("/");
                        println!("      Direccion [{fmt_addr}]: {params:?}");
                    }
                }
                Err(e) => print_err(&format!("Fallo en BBVI: {e}")),
            }
        }
        Algorithm::Enumeration => {
            println!("Explorando todos los estados posibles...");
            match enumerate_traces(model, ENUM_MAX_TRACES) {
                Ok(runs) => {
                    let (mut pmf, log_z) = posterior_table(&runs);

                    pmf.sort_by(|a, b| a.0.as_i64().cmp(&b.0.as_i64()));

                    print_ok(&format!("Enumeracion completada. Log Evidence (Z): {log_z:.4}"));
                    print_ok(&format!("Estados explorados totales: {}", runs.len()));

                    let mut table = Table::builder().style(TableStyle::elegant()).build();

                    table.add_row(Row::new(vec![
                        TableCell::builder("Valor")
                            .alignment(Alignment::Center)
                            .build(),
                        TableCell::builder("Prob")
                            .alignment(Alignment::Center)
                            .build(),
                        TableCell::builder("Log mass")
                            .alignment(Alignment::Right)
                            .build(),
                    ]));

                    for (val, prob, lw) in pmf {
                        let prob_str = if prob < 0.0001 && prob > 0.0 {
                            format!("{:.4e}", prob)
                        } else {
                            format!("{:.4}", prob)
                        };

                        table.add_row(Row::new(vec![
                            TableCell::builder(format!("{}", val))
                                .alignment(Alignment::Center)
                                .build(),
                            TableCell::builder(prob_str)
                                .alignment(Alignment::Center)
                                .build(),
                            TableCell::builder(fmt_log_mass(lw))
                                .alignment(Alignment::Right)
                                .build(),
                        ]));
                    }

                    let table_str = table.render();

                    for line in table_str.lines() {
                        println!("  {}", line)
                    }
                }
                Err(e) => print_err(&format!("Fallo: {e}")),
            }
        }
    }

    println!("   (tiempo: {:.2?})", start.elapsed());
}

fn print_usage(demos: &[Demo]) {
    eprintln!("Uso:");
    eprintln!("  cargo run                                -> corre todas las demos hardcodeadas");
    eprintln!(
        "  cargo run -- <numero>                    -> corre una demo especifica (1-{})",
        demos.len()
    );
    eprintln!("  cargo run -- <archivo.hoppl>             -> corre un modelo deterministico (sin sample/observe)");
    eprintln!("  cargo run -- <archivo.hoppl> <algoritmo> -> corre un modelo probabilistico con el algoritmo dado");
    eprintln!();
    eprintln!(
        "Algoritmos disponibles: lw, ssmh, smc, bbvi, exact-enumeration (alias: enum, exact)"
    );
    eprintln!();
    eprintln!("Demos disponibles:");
    for d in demos {
        eprintln!("   {}: {}", d.id, d.label);
    }
}

fn main() {
    println!("Iniciando Demostracion: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Autor: Martin Nievas Wilberger");

    let args: Vec<String> = env::args().collect();

    let demos: Vec<Demo> = vec![
        Demo {
            id: 1,
            label: "Likelihood Weighting",
            run: demo_lw,
        },
        Demo {
            id: 2,
            label: "Sequential Monte Carlo",
            run: demo_smc,
        },
        Demo {
            id: 3,
            label: "Seguridad SMC (analisis estatico)",
            run: demo_smc_safety,
        },
        Demo {
            id: 4,
            label: "Single-Site MH",
            run: demo_ssmh,
        },
        Demo {
            id: 5,
            label: "BBVI",
            run: demo_bbvi,
        },
        Demo {
            id: 6,
            label: "Exact Enumeration",
            run: demo_enum,
        },
    ];

    // Modo archivo + algoritmo: cargo run -- <archivo.hoppl> <algoritmo>
    if args.len() >= 3 && args[1].parse::<usize>().is_err() {
        let file_path = &args[1];
        let algo_name = &args[2];

        let algorithm = match Algorithm::parse(algo_name) {
            Some(a) => a,
            None => {
                print_err(&format!("Algoritmo desconocido: '{algo_name}'"));
                print_usage(&demos);
                std::process::exit(1);
            }
        };

        let model = load_model_file(file_path);
        let mut rng = StdRng::seed_from_u64(42);
        run_algorithm_on_model(algorithm, &model, &mut rng);
        return;
    }

    // Modo archivo determinístico: cargo run -- <archivo.hoppl>  (sin algoritmo)
    if args.len() == 2 && args[1].parse::<usize>().is_err() {
        let file_path = &args[1];
        let model = load_model_file(file_path);
        run_deterministic_model(file_path, &model);
        return;
    }

    let selected: Option<usize> = args.get(1).and_then(|s| s.parse().ok());

    if let Some(n) = selected {
        if !demos.iter().any(|d| d.id == n) {
            eprintln!(
                "Numero de demo invalido: {n}. Usa un valor entre 1 y {}.",
                demos.len()
            );
            print_usage(&demos);
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

        let mut demo_rng = StdRng::seed_from_u64(42 );
        (demo.run)(&mut demo_rng);

        if selected.is_none() {
            pause();
        }
    }

    print_header("FIN DE LA DEMOSTRACION - HOPPL EN RUST");
    println!("Tiempo total: {:.2?}", total_start.elapsed());

    if selected.is_none() {
        println!(
            "Tip: ejecuta `cargo run -- <numero>` para correr una sola demo (1-{}).",
            demos.len()
        );
        println!(
            "Tip: ejecuta `cargo run -- <archivo.hoppl>` para correr un modelo deterministico propio."
        );
        println!(
            "Tip: ejecuta `cargo run -- <archivo.hoppl> <algoritmo>` para correr un modelo probabilistico propio."
        );
    }
}