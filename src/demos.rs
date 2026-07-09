/*

Modulo que implementa las demos hardcodeadas, la funcion que construye el vector de demostraciones para
la ejecución.

*/

use std::time::Instant;

use rand::rngs::StdRng;

use ppl_tp_final::inference::bbvi::run_bbvi;
use ppl_tp_final::inference::exact_enumeration::{enumerate_traces, posterior_table};
use ppl_tp_final::inference::lw::likelihood_weighting;
use ppl_tp_final::inference::smc::run_smc;
use ppl_tp_final::inference::ssmh::single_site_mh;

use term_table::{row::Row, table_cell::*, Table, TableStyle};

use crate::stats::{effective_sample_size, mcmc_mean_std_err_ess, sample_mean_std_err, weighted_mean_var};
use crate::ui::{fmt_log_mass, print_err, print_header, print_ok, print_warn};

pub struct Demo {
    pub id: usize,
    pub label: &'static str,
    pub run: fn(&mut StdRng),
}

pub fn build_demos() -> Vec<Demo> {
    vec![
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
    ]
}

pub fn run_demo<F: FnOnce()>(title: &str, model: &str, f: F) {
    print_header(title);
    println!("Modelo ejecutado:\n{}", model.trim());
    println!();
    let start = Instant::now();
    f();
    println!("   (tiempo: {:.2?})", start.elapsed());
}

pub fn demo_lw(rng: &mut StdRng) {
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

pub fn demo_smc(rng: &mut StdRng) {
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

pub fn demo_smc_safety(rng: &mut StdRng) {
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
                    print_ok(
                        "Analisis Estatico exitoso. Ejecucion abortada antes de instanciar particulas.",
                    );
                    println!("   Detalle del Linter:\n     >> {e}");
                }
            }
        },
    );
}

pub fn demo_ssmh(rng: &mut StdRng) {
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

pub fn demo_bbvi(rng: &mut StdRng) {
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

                println!(
                    "\n   Nota: al observar 2.0, 2.5 y 2.1, Adam ajusta la media (mu) hacia ~2.2000."
                );
            }
            Err(e) => print_err(&format!("Fallo en BBVI: {e}")),
        }
    });
}

pub fn demo_enum(_rng: &mut StdRng) {
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