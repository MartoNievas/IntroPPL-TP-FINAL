/*

Modulo que implementa los distintos modos de ejecución del lenguaje:

    - Demostraciones completas
    - Demostracion particular
    - Archivo no deterministico
    - Archivo deterministico

*/

use std::fs;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;

use ppl_tp_final::inference::bbvi::run_bbvi;
use ppl_tp_final::inference::exact_enumeration::{enumerate_traces, posterior_table};
use ppl_tp_final::inference::lw::likelihood_weighting;
use ppl_tp_final::inference::smc::run_smc;
use ppl_tp_final::inference::ssmh::single_site_mh;

use term_table::{row::Row, table_cell::*, Table, TableStyle};

use crate::cli::{print_usage, Algorithm, Config};
use crate::demos::build_demos;
use crate::interpreter::{initial_machine, resume, Msg};
use crate::stats::{
    effective_sample_size, is_numeric, mcmc_mean_std_err_ess, print_categorical_unweighted,
    print_categorical_weighted, sample_mean_std_err, weighted_mean_var,
};
use crate::ui::{fmt_log_mass, pause, print_err, print_header, print_ok, print_warn};

/// Unico punto de entrada de ejecucion: recibe la Config ya validada por
/// `cli::Config::parse_args` y decide que correr. `main` no necesita saber
/// nada mas que esto.
pub fn run(config: Config) {
    match config {
        Config::Invalid(msg) => {
            eprintln!("{msg}");
            print_usage(&build_demos());
            std::process::exit(1);
        }
        Config::Demo(selected) => run_demos(selected),
        Config::Deterministic(file_path) => {
            let model = load_model_file(&file_path);
            run_deterministic_model(&file_path, &model);
        }
        Config::File(file_path, algorithm) => {
            let model = load_model_file(&file_path);
            let mut rng = StdRng::seed_from_u64(42);
            run_algorithm_on_model(algorithm, &model, &mut rng);
        }
    }
}

fn run_demos(selected: Option<usize>) {
    let demos = build_demos();
    let total_start = Instant::now();

    for demo in &demos {
        if let Some(n) = selected {
            if n != demo.id {
                continue;
            }
        }

        let mut demo_rng = StdRng::seed_from_u64(42);
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
            print_ok(&format!("Resultado: {}", value));
        }

        Ok(Msg::Sample(addr, _, _)) => {
            print_err(&format!(
                "El programa no es deterministico: se encontro un 'sample' en la direccion {addr:?}."
            ));
            println!(
                "   Este modo es solo para programas sin 'sample'/'observe'. Corre con un algoritmo de inferencia en su lugar:"
            );
            println!(
                "      cargo run -- {file_path} <algoritmo>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }

        Ok(Msg::Observe(addr, _, _, _)) => {
            print_err(&format!(
                "El programa no es deterministico: se encontro un 'observe' en la direccion {addr:?}."
            ));
            println!(
                "   Este modo es solo para programas sin 'sample'/'observe'. Corre con un algoritmo de inferencia en su lugar:"
            );
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
                    if vals.iter().all(is_numeric) {
                        let (mean, var) = weighted_mean_var(&vals, &weights);
                        let std_err = (var / N_PARTICLES_LW as f64).sqrt();
                        let ess = effective_sample_size(&weights);
                        print_ok(&format!(
                            "Media a posteriori estimada: {mean:.4} ± {std_err:.4}"
                        ));
                        print_ok(&format!(
                            "Effective Sample Size (ESS): {ess:.1} / {N_PARTICLES_LW}"
                        ));
                    } else {
                        print_categorical_weighted(&vals, &weights);
                    }
                }
                Err(e) => print_err(&format!("Fallo en Likelihood Weighting: {e}")),
            }
        }
        Algorithm::Smc => {
            println!("Ejecutando SMC con {N_PARTICLES_SMC} particulas sincronizadas...");
            match run_smc(model, N_PARTICLES_SMC, rng) {
                Ok(vals) => {
                    if vals.iter().all(is_numeric) {
                        let (mean, std_err) = sample_mean_std_err(&vals);
                        print_ok(&format!(
                            "Valor esperado estimado: {mean:.4} ± {std_err:.4}"
                        ));
                    } else {
                        print_categorical_unweighted(&vals);
                    }
                }
                Err(e) => print_err(&format!("Fallo en SMC: {e}")),
            }
        }
        Algorithm::Ssmh => {
            println!("Ejecutando SSMH (Pasos: {SSMH_STEPS}, Warmup: {SSMH_WARMUP})...");
            match single_site_mh(model, rng, SSMH_STEPS, SSMH_WARMUP) {
                Ok(chain) => {
                    if chain.iter().all(is_numeric) {
                        let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);
                        print_ok(&format!("Valor estimado: {mean:.4} ± {std_err:.4}"));
                        print_ok(&format!(
                            "Effective Sample Size (ESS, por autocorrelacion): {ess:.1} / {SSMH_STEPS}"
                        ));
                    } else {
                        print_categorical_unweighted(&chain);
                    }
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

                    print_ok(&format!(
                        "Enumeracion completada. Log Evidence (Z): {log_z:.4}"
                    ));
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