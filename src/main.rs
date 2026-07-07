mod inference;
mod interpreter;
mod parser;

use std::env;
use std::fs;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;
use PPL_TP_FINAL::inference::bbvi::run_bbvi;
use PPL_TP_FINAL::inference::exact_enumeration::{enumerate_traces, posterior_table};
use PPL_TP_FINAL::inference::lw::likelihood_weighting;
use PPL_TP_FINAL::inference::smc::run_smc;
use PPL_TP_FINAL::inference::ssmh::single_site_mh;
use PPL_TP_FINAL::parser::value::RVal;
use term_table::{row::Row, table_cell::*, Table, TableStyle};

// ─ Colores ANSI para que la salida se lea mejor en una presentación en vivo
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";

// Extrae el valor f64 de un RVal numérico.
fn as_f64(val: &RVal) -> f64 {
    match val {
        RVal::Float(f) => *f,
        RVal::Int(i) => *i as f64,
        _ => panic!("Se esperaba un valor numerico, se obtuvo: {val:?}"),
    }
}

// funcion auxiliar para pausar la ejecucion luego de cada demostracion
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

// Imprime encabezado + modelo fuente, corre el bloque de la demo,
// y reporta cuánto tardó. No hace panic si `f` maneja sus propios
// errores internamente en las demos abajo.
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

    run_demo(
        "2. SEQUENTIAL MONTE CARLO (SMC / Filtro de Particulas)",
        model,
        || {
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
        },
    );
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
                let var: f64 = chain
                    .iter()
                    .map(|x| (as_f64(x) - mean).powi(2))
                    .sum::<f64>()
                    / (steps as f64);
                let std_err = (var / steps as f64).sqrt();

                print_ok(&format!(
                    "Pendiente 'm' estimada: {mean:.4} ± {std_err:.4} (Esperada: 2.0)"
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

        println!(
            "Optimizando ELBO con Adam (Pasos: {steps}, Muestras/Lote: {n_samples}, LR: {lr})..."
        );

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
                let (mut pmf, log_z) = posterior_table(&runs);
                print_ok(&format!(
                    "Enumeracion completada. Log Evidence (Z): {log_z:.4}"
                ));

                // 1. Ordenamos la tabla para que se vea profesional (de menor a mayor valor)
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
                        TableCell::builder(format!("{:.4}", log_mass))
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

// ============================================================================
// MODO ARCHIVO: cargo run -- <archivo.hoppl> <algoritmo>
// ============================================================================

/// Algoritmos soportados en modo archivo.
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
        // Se acepta "exact enumeration" (con espacio) tal como lo pide el enunciado,
        // ademas de variantes mas comodas para la terminal (sin espacios).
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

// Lee el modelo desde el archivo .hoppl indicado. Termina el programa
// con un mensaje claro si el archivo no existe o no se puede leer.
fn load_model_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            print_err(&format!("No se pudo leer el archivo '{path}': {e}"));
            std::process::exit(1);
        }
    }
}

// Corre el algoritmo seleccionado sobre un modelo arbitrario cargado desde
// disco. A diferencia de las demos hardcodeadas, aca no conocemos el valor
// analitico esperado, asi que solo reportamos datos estadisticos generales.
fn run_algorithm_on_model(algorithm: Algorithm, model: &str, rng: &mut StdRng) {
    // Parametros por defecto (mismos que usan las demos hardcodeadas).
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
                    let mean: f64 = vals
                        .iter()
                        .zip(weights.iter())
                        .map(|(v, w)| as_f64(v) * w)
                        .sum();
                    print_ok(&format!("Media a posteriori estimada: {mean:.4}"));
                }
                Err(e) => print_err(&format!("Fallo en Likelihood Weighting: {e}")),
            }
        }
        Algorithm::Smc => {
            println!("Ejecutando SMC con {N_PARTICLES_SMC} particulas sincronizadas...");
            match run_smc(model, N_PARTICLES_SMC, rng) {
                Ok(vals) => {
                    let mean: f64 = vals.iter().map(as_f64).sum::<f64>() / (N_PARTICLES_SMC as f64);
                    print_ok(&format!("Valor esperado estimado: {mean:.4}"));
                }
                Err(e) => print_err(&format!("Fallo en SMC: {e}")),
            }
        }
        Algorithm::Ssmh => {
            println!("Ejecutando SSMH (Pasos: {SSMH_STEPS}, Warmup: {SSMH_WARMUP})...");
            match single_site_mh(model, rng, SSMH_STEPS, SSMH_WARMUP) {
                Ok(chain) => {
                    let mean: f64 = chain.iter().map(as_f64).sum::<f64>() / (SSMH_STEPS as f64);
                    let var: f64 = chain
                        .iter()
                        .map(|x| (as_f64(x) - mean).powi(2))
                        .sum::<f64>()
                        / (SSMH_STEPS as f64);
                    let std_err = (var / SSMH_STEPS as f64).sqrt();
                    print_ok(&format!("Valor estimado: {mean:.4} ± {std_err:.4}"));
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

                    println!("\nResultados de la Optimizacion Variacional:");
                    println!("   ELBO Inicial : {elbo_inicial:.4e}");
                    println!("   ELBO Final   : {elbo_final:.4e}");

                    if elbo_final > elbo_inicial {
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

                    print_ok("Enumeracion completada. Log Evidence (Z): {log_z:.4}");
                    print_ok(&format!("Estados explorados totales: {}", runs.len()));

                    let mut table = Table::builder().style(TableStyle::elegant()).build();

                    // Encabezado
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
                            TableCell::builder(format!("{:.4}", lw))
                                .alignment(Alignment::Right)
                                .build(),
                        ]));
                    }

                    let table_str = table.render();

                    for line in table_str.lines() {
                        println!("  {}",line)
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
    eprintln!("  cargo run                              -> corre todas las demos hardcodeadas");
    eprintln!(
        "  cargo run -- <numero>                  -> corre una demo especifica (1-{})",
        demos.len()
    );
    eprintln!("  cargo run -- <archivo.hoppl> <algoritmo> -> corre un modelo propio");
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

// ============================================================================
// MAIN
// ============================================================================
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

    // ── Caso 1: cargo run -- <archivo.hoppl> <algoritmo> ──────────────────
    // Se detecta porque hay 2+ argumentos y el primero NO es un numero.
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

    // ── Caso 2: cargo run -- <numero> -> corre una demo especifica ────────
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
    } else if args.len() >= 2 {
        // Se paso un solo argumento que no es un numero valido de demo
        // (y no llego a matchear el caso de archivo+algoritmo, que requiere 2 args).
        print_err(&format!("Argumento invalido: '{}'.", args[1]));
        print_usage(&demos);
        std::process::exit(1);
    }

    // ── Caso 3: cargo run (sin argumentos) -> corre todas las demos ───────
    let total_start = Instant::now();

    for demo in &demos {
        if let Some(n) = selected {
            if n != demo.id {
                continue;
            }
        }

        let mut demo_rng = StdRng::seed_from_u64(42 + demo.id as u64);
        (demo.run)(&mut demo_rng);

        // Llamamos a pause si el usuario NO seleccionó una demo única
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
            "Tip: ejecuta `cargo run -- <archivo.hoppl> <algoritmo>` para correr un modelo propio."
        );
    }
}

