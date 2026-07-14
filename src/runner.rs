/*

Module that implements the different execution modes of the language:

    - Full demonstrations
    - Single demonstration
    - Non-deterministic file
    - Deterministic file
    - Non-deterministic file in debug mode.

*/

use std::fs;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::inference::bbvi::run_bbvi;
use crate::inference::exact_enumeration::{enumerate_traces, posterior_table};
use crate::inference::lw::likelihood_weighting;
use crate::inference::smc::run_smc;
use crate::inference::ssmh::single_site_mh;

use term_table::{row::Row, table_cell::*, Table, TableStyle};

use crate::cli::{print_usage, Algorithm, Config};
use crate::demos::build_demos;
use crate::interpreter::{initial_machine, resume, Msg};
use crate::stats::{
    ci95_margin, effective_sample_size, is_numeric, mcmc_mean_std_err_ess,
    print_categorical_unweighted, print_categorical_weighted, sample_mean_std_err,
    weighted_mean_var,
};
use crate::ui::{fmt_log_mass, pause, print_err, print_header, print_ok, print_warn};

// Threshold (in percent) below which we warn about likely particle/sample
// degeneracy. Shared by LW's ESS% and SSMH's acceptance rate diagnostics.
const LOW_DIAGNOSTIC_THRESHOLD_PCT: f64 = 10.0;
// Upper threshold for SSMH's acceptance rate: if the chain accepts almost
// every proposal, it's usually a sign the proposal distribution is too
// conservative and under-explores the posterior.
const HIGH_ACCEPTANCE_THRESHOLD_PCT: f64 = 90.0;

// Single execution entry point: receives the Config already validated by
// `cli::Config::parse_args` and decides what to run. `main` doesn't need to
// know anything beyond this.
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
        Config::Debug(file_path, algorithm) => {
            //let model = load_model_file(&file_path);
            //let mut rng = StdRng::seed_from_u64(42);
            println!("Debug mode active");
            //run_debug_term(&model, algorithm, rng);
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

    print_header("END OF DEMONSTRATION - HOPPL IN RUST");
    println!("Total time: {:.2?}", total_start.elapsed());

    if selected.is_none() {
        println!(
            "Tip: run `cargo run -- <number>` to run a single demo (1-{}).",
            demos.len()
        );
        println!(
            "Tip: run `cargo run -- <file.hoppl>` to run your own deterministic model."
        );
        println!(
            "Tip: run `cargo run -- <file.hoppl> <algorithm>` to run your own probabilistic model."
        );
    }
}

fn load_model_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            print_err(&format!("Could not read file '{path}': {e}"));
            std::process::exit(1);
        }
    }
}

// Runs a HOPPL program assumed to be **deterministic** (no 'sample' or
// 'observe'): instead of invoking an inference engine, it advances the CEK
// machine directly until it finishes. If the program turns out to have
// probabilistic effects, this is detected as soon as the first one appears
// and reported with a clear message instead of failing in a confusing way.
fn run_deterministic_model(file_path: &str, model: &str) {
    print_header("FILE MODE: Deterministic Execution");
    println!("Model loaded:\n{}", model.trim());
    println!();

    let start = Instant::now();

    let machine = match initial_machine(model) {
        Ok(m) => m,
        Err(e) => {
            print_err(&format!("Error initializing the program: {e}"));
            println!("   (time: {:.2?})", start.elapsed());
            return;
        }
    };

    match resume(machine) {
        Ok(Msg::Done(value, _)) => {
            print_ok(&format!("Result: {}", value));
        }

        Ok(Msg::Sample(addr, _, _)) => {
            print_err(&format!(
                "The program is not deterministic: found a 'sample' at address {addr:?}."
            ));
            println!(
                "   This mode is only for programs without 'sample'/'observe'/'factor'. Run with an inference algorithm instead:"
            );
            println!(
                "      cargo run -- {file_path} <algorithm>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }

        Ok(Msg::Factor(addr, _ ,_ )) => {
            print_err(&format!(
                "The program is not deterministic: found an 'factor' at address {addr:?}."
            ));
            println!(
                "   This mode is only for programs without 'sample'/'observe'/'factor'. Run with an inference algorithm instead:"
            );
            println!(
                "      cargo run -- {file_path} <algorithm>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }

        Ok(Msg::Observe(addr, _, _, _)) => {
            print_err(&format!(
                "The program is not deterministic: found an 'observe' at address {addr:?}."
            ));
            println!(
                "   This mode is only for programs without 'sample'/'observe'/'factor'. Run with an inference algorithm instead:"
            );
            println!(
                "      cargo run -- {file_path} <algorithm>   (lw | ssmh | smc | bbvi | exact-enumeration)"
            );
        }
        Err(e) => print_err(&format!("Execution error: {e}")),
    }

    println!("   (time: {:.2?})", start.elapsed());
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

    print_header(&format!("FILE MODE: {}", algorithm.label()));
    println!("Model loaded:\n{}", model.trim());
    println!();

    let start = Instant::now();

    match algorithm {
        Algorithm::Lw => {
            println!("  Running LW with {N_PARTICLES_LW} particles...");
            match likelihood_weighting(model, N_PARTICLES_LW, rng) {
                Ok((vals, weights)) => {
                    if vals.iter().all(is_numeric) {
                        let (mean, var) = weighted_mean_var(&vals, &weights);
                        let std_err = (var / N_PARTICLES_LW as f64).sqrt();
                        let margin = ci95_margin(std_err);
                        let ess = effective_sample_size(&weights);
                        let ess_pct = 100.0 * ess / N_PARTICLES_LW as f64;

                        print_ok(&format!(
                            "Estimated posterior mean: {mean:.4} ± {std_err:.4}"
                        ));
                        print_ok(&format!(
                            "95% CI: [{:.4}, {:.4}]",
                            mean - margin,
                            mean + margin
                        ));
                        print_ok(&format!(
                            "Effective Sample Size (ESS): {ess:.1} / {N_PARTICLES_LW} ({ess_pct:.1}%)"
                        ));
                        if ess_pct < LOW_DIAGNOSTIC_THRESHOLD_PCT {
                            print_warn(
                                "Low ESS: particles may be degenerating. Consider increasing N or reviewing the model.",
                            );
                        }
                    } else {
                        print_categorical_weighted(&vals, &weights);
                    }
                }
                Err(e) => print_err(&format!("Failure in Likelihood Weighting: {e}")),
            }
        }
        Algorithm::Smc => {
            println!("Running SMC with {N_PARTICLES_SMC} synchronized particles...");
            match run_smc(model, N_PARTICLES_SMC, rng) {
                Ok(vals) => {
                    if vals.iter().all(is_numeric) {
                        let (mean, std_err) = sample_mean_std_err(&vals);
                        let margin = ci95_margin(std_err);
                        print_ok(&format!(
                            "Estimated expected value: {mean:.4} ± {std_err:.4}"
                        ));
                        print_ok(&format!(
                            "95% CI: [{:.4}, {:.4}]",
                            mean - margin,
                            mean + margin
                        ));
                    } else {
                        print_categorical_unweighted(&vals);
                    }
                }
                Err(e) => print_err(&format!("Failure in SMC: {e}")),
            }
        }
        Algorithm::Ssmh => {
            println!("Running SSMH (Steps: {SSMH_STEPS}, Warmup: {SSMH_WARMUP})...");
            match single_site_mh(model, rng, SSMH_STEPS, SSMH_WARMUP) {
                Ok((chain, acceptance_rate)) => {
                    if chain.iter().all(is_numeric) {
                        let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);
                        let margin = ci95_margin(std_err);
                        print_ok(&format!("Estimated value: {mean:.4} ± {std_err:.4}"));
                        print_ok(&format!(
                            "95% CI: [{:.4}, {:.4}]",
                            mean - margin,
                            mean + margin
                        ));
                        print_ok(&format!(
                            "Effective Sample Size (ESS, via autocorrelation): {ess:.1} / {SSMH_STEPS}"
                        ));
                    } else {
                        print_categorical_unweighted(&chain);
                    }

                    if acceptance_rate.is_nan() {
                        print_warn(
                            "Acceptance rate: N/A (model has no probabilistic 'sample' sites to propose on)",
                        );
                    } else {
                        let acc_pct = 100.0 * acceptance_rate;
                        print_ok(&format!("Acceptance rate: {acc_pct:.1}%"));
                        if acc_pct < LOW_DIAGNOSTIC_THRESHOLD_PCT {
                            print_warn(
                                "Low acceptance rate: the chain is barely moving. Consider reviewing the model or the proposal.",
                            );
                        } else if acc_pct > HIGH_ACCEPTANCE_THRESHOLD_PCT {
                            print_warn(
                                "Very high acceptance rate: the chain may be under-exploring the posterior (proposals too conservative).",
                            );
                        }
                    }
                }
                Err(e) => print_err(&format!("Failure in SSMH: {e}")),
            }
        }
        Algorithm::Bbvi => {
            println!(
                "Optimizing ELBO with Adam (Steps: {BBVI_STEPS}, Samples/Batch: {BBVI_SAMPLES}, LR: {BBVI_LR})..."
            );
            match run_bbvi(model, BBVI_STEPS, BBVI_SAMPLES, BBVI_LR, rng) {
                Ok((elbo_history, theta_opt)) => {
                    let elbo_inicial = elbo_history.first().unwrap();
                    let elbo_final = elbo_history.last().unwrap();
                    let delta = elbo_final - elbo_inicial;

                    println!("\nResults of the Variational Optimization:");
                    println!("   Initial ELBO : {elbo_inicial:.4}");
                    println!("   Final ELBO   : {elbo_final:.4}");
                    println!("   Delta ELBO   : {delta:+.4}");

                    if delta > 0.0 {
                        print_ok(
                            "The ELBO ascended successfully. The optimizer reduced the divergence.",
                        );
                    } else {
                        print_warn("The ELBO did not rise significantly.");
                    }

                    println!("\n   Optimized Variational Parameters (Theta):");
                    for (addr, params) in theta_opt {
                        let fmt_addr = addr.join("/");
                        println!("      Address [{fmt_addr}]: {params:?}");
                    }
                }
                Err(e) => print_err(&format!("Failure in BBVI: {e}")),
            }
        }
        Algorithm::Enumeration => {
            println!("Exploring all possible states...");
            match enumerate_traces(model, ENUM_MAX_TRACES) {
                Ok(runs) => {
                    let (mut pmf, log_z) = posterior_table(&runs);

                    pmf.sort_by(|a, b| a.0.as_i64().cmp(&b.0.as_i64()));

                    print_ok(&format!(
                        "Enumeration complete. Log Evidence (Z): {log_z:.4}"
                    ));
                    print_ok(&format!("Total states explored: {}", runs.len()));

                    let mut table = Table::builder().style(TableStyle::elegant()).build();

                    table.add_row(Row::new(vec![
                        TableCell::builder("Value")
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
                Err(e) => print_err(&format!("Failure: {e}")),
            }
        }
    }

    println!("   (time: {:.2?})", start.elapsed());
}