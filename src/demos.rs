/*

Module that implements the hardcoded demos, and the function that builds the vector of
demonstrations for execution.

*/

use std::time::Instant;

use rand::rngs::StdRng;
use rand_distr::num_traits::Float;

use crate::inference::bbvi::run_bbvi;
use crate::inference::exact_enumeration::{enumerate_traces, posterior_table};
use crate::inference::lw::likelihood_weighting;
use crate::inference::smc::run_smc;
use crate::inference::ssmh::single_site_mh;

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
            label: "SMC Safety (static analysis)",
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
        Demo {
            id: 7,
            label: "Factor (soft conditioning)",
            run: demo_factor,
        },
    ]
}

pub fn run_demo<F: FnOnce()>(title: &str, model: &str, f: F) {
    print_header(title);
    println!("Model executed:\n{}", model.trim());
    println!();
    let start = Instant::now();
    f();
    println!("   (time: {:.2?})", start.elapsed());
}

pub fn demo_lw(rng: &mut StdRng) {
    let model = r#"
        ; Prior: mu ~ Normal(0, 1)
        ; Likelihood: We observe the value 3.0 from Normal(mu, 1)
        (let [mu (sample (normal 0 1))]
            (observe (normal mu 1) 3.0)
            mu)
    "#;

    run_demo("1. LIKELIHOOD WEIGHTING (LW)", model, || {
        let n_particles = 5000;
        println!("Running LW with {n_particles} particles...");

        match likelihood_weighting(model, n_particles, rng) {
            Ok((vals, weights)) => {
                let (mean, var) = weighted_mean_var(&vals, &weights);
                let std_err = (var / n_particles as f64).sqrt();
                let ess = effective_sample_size(&weights);
                print_ok(&format!(
                    "Estimated posterior mean: {mean:.4} ± {std_err:.4} (Analytical: ~1.5000)"
                ));
                print_ok(&format!(
                    "Effective Sample Size (ESS): {ess:.1} / {n_particles}"
                ));
            }
            Err(e) => print_err(&format!("Failure in Likelihood Weighting: {e}")),
        }
    });
}

pub fn demo_smc(rng: &mut StdRng) {
    let model = r#"
        ; Prior: p ~ Beta(2, 2)
        ; Likelihood: We observe 3 heads (true) and 1 tails (false)
        (let [p (sample (beta 2.0 2.0))]
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) false)
            p)
    "#;

    run_demo(
        "2. SEQUENTIAL MONTE CARLO (SMC / Particle Filter)",
        model,
        || {
            let n_particles = 2000;
            println!("Running SMC with {n_particles} synchronized particles...");

            match run_smc(model, n_particles, rng) {
                Ok(vals) => {
                    let (mean, std_err) = sample_mean_std_err(&vals);
                    print_ok(&format!(
                        "Estimated probability 'p': {mean:.4} ± {std_err:.4} (Analytical: ~0.6250)"
                    ));
                }
                Err(e) => print_err(&format!("Failure in SMC: {e}")),
            }
        },
    );
}

pub fn demo_smc_safety(rng: &mut StdRng) {
    let model = r#"
        ; This program breaks SMC because the 'observe' is hidden inside a
        ; stochastic branch, which desynchronizes the particles at runtime.
        (if (sample (bernoulli 0.5))
            (observe (normal 0 1) 1.0)
            0)
    "#;

    run_demo(
        "3. SMC SECURITY: STATIC TRACE ANALYSIS",
        model,
        || {
            println!("Analyzing AST and preventing desynchronized execution...");

            match run_smc(model, 100, rng) {
                Ok(_) => {
                    print_err("Security failure: the model should have been rejected by the linter.")
                }
                Err(e) => {
                    print_ok(
                        "Static analysis successful. Execution aborted before instantiating particles..",
                    );
                    println!("   Linter Details:\n     >> {e}");
                }
            }
        },
    );
}

pub fn demo_ssmh(rng: &mut StdRng) {
    let model = r#"
        ; Prior for slope (m) and bias (b)
        (let [m (sample (normal 0 2))
              b (sample (normal 0 2))]
            ; We observe noisy points from the line y = 2x + 1
            (observe (normal (+ (* m 1) b) 0.5) 3.0)
            (observe (normal (+ (* m 2) b) 0.5) 5.0)
            m)
    "#;

    run_demo("4. SINGLE-SITE METROPOLIS-HASTINGS (MCMC)", model, || {
        let steps = 4000;
        let warmup = 1000;
        println!("Running SSMH (Steps: {steps}, Warmup: {warmup})...");

        match single_site_mh(model, rng, steps, warmup) {
            Ok(chain) => {
                let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);

                print_ok(&format!(
                    "Estimated slope 'm': {mean:.4} ± {std_err:.4} (Expected: 2.0)"
                ));
                print_ok(&format!(
                    "Effective Sample Size (ESS, via autocorrelation): {ess:.1} / {steps}"
                ));
            }
            Err(e) => print_err(&format!("Failure in SSMH: {e}")),
        }
    });
}

pub fn demo_bbvi(rng: &mut StdRng) {
    let model = r#"
        ; Prior: mu ~ Normal(0.0, 5.0) (wide uncertainty)
        ; Likelihood: We observe 3 measurements pointing to a mean of ~2.2
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
            "Optimizing ELBO with Adam (Steps: {steps}, Samples/Batch: {n_samples}, LR: {lr})..."
        );

        match run_bbvi(model, steps, n_samples, lr, rng) {
            Ok((elbo_history, theta_opt)) => {
                let elbo_inicial = elbo_history.first().unwrap();
                let elbo_final = elbo_history.last().unwrap();
                let delta = elbo_final - elbo_inicial;

                println!("\nResults of Variational Optimization:");
                println!("   Initial ELBO : {elbo_inicial:.4}");
                println!("   Final ELBO   : {elbo_final:.4}");
                println!("   Delta ELBO   : {delta:+.4}");

                if delta > 0.0 {
                    print_ok("ELBO has successfully ascended. The optimizer reduced the divergence.");
                } else {
                    print_warn("The ELBO did not rise significantly.");
                }

                println!("\n   Optimized Variational Parameters (Theta):");
                for (addr, params) in theta_opt {
                    let fmt_addr = addr.join("/");
                    println!("      Address [{fmt_addr}]: {params:?}");
                }

                println!(
                    "\n   Note: Upon observing 2.0, 2.5 and 2.1, Adam adjusts the mean (mu) towards ~2.2000."
                );
            }
            Err(e) => print_err(&format!("Failure in BBVI: {e}")),
        }
    });
}

pub fn demo_enum(_rng: &mut StdRng) {
    let model = r#"
        ; A parent has genotype 0, 1, or 2 (copies of an allele).
        ; Prior: genotype ~ DiscreteUniform(0, 3)
        ; If the genotype is 1, there is a 50% chance of passing on the gene.
        (let [genotype (sample (uniform-discrete 0 3))
              p_expression (if (= genotype 0) 0.0
                           (if (= genotype 1) 0.5 1.0))
              expressed (sample (bernoulli p_expression))]
            (observe (bernoulli p_expression) true)
            genotype)
    "#;

    run_demo("6. EXACT ENUMERATION", model, || {
        println!("Exploring all possible states...");

        match enumerate_traces(model, 1000) {
            Ok(runs) => {
                let (mut pmf, log_z) = posterior_table(&runs);
                print_ok(&format!(
                    "Enumeration complete. Log Evidence (Z): {log_z:.4}"
                ));

                pmf.sort_by(|a, b| a.0.as_i64().cmp(&b.0.as_i64()));

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
            Err(e) => print_err(&format!("Failure in Exact Enumeration: {e}")),
        }
    });
}

pub fn demo_factor(rng: &mut StdRng) {
    let model = r#"
        ; Prior: mu ~ Normal(0, 10) (wide, weakly informative)
        ; Instead of (observe (normal mu 1.0) 3.0), we compute the Gaussian
        ; log-density by hand and add it with factor. Both approaches should
        ; converge to (approximately) the same posterior.
        (let [mu (sample (normal 0.0 10.0))
              diff (- mu 3.0)
              log_lik (* -0.5 (* diff diff))]
            (factor log_lik)
            mu)
    "#;

    run_demo("7. FACTOR (SOFT CONDITIONING)", model, || {
        // factor(x) is unnormalized: it omits the Gaussian's
        // -0.5*log(2*pi*sigma^2) constant. That constant is the same for
        // every particle/step, so it cancels out in the posterior mean
        // -- it would only shift the log evidence, which these algorithms
        // don't report here. That's why the analytical mean below still
        // matches a proper Normal(0,10) prior combined with a
        // Normal(mu, 1.0) likelihood at 3.0, via precision weighting.
        let analytical_mean = 3.0 / (1.0 + 1.0 / 100.0);
        let analytical_std = (1.0 / (1.0 + 1.0 / 100.0)).sqrt();

        println!(
            "Analytical posterior (conjugate Normal-Normal): mean ~{analytical_mean:.4}, std ~{analytical_std:.4}"
        );
        println!();

        let n_particles = 5000;
        println!("Running LW with {n_particles} particles...");
        match likelihood_weighting(model, n_particles, rng) {
            Ok((vals, weights)) => {
                let (mean, var) = weighted_mean_var(&vals, &weights);
                let std_err = (var / n_particles as f64).sqrt();
                let ess = effective_sample_size(&weights);
                print_ok(&format!(
                    "[LW]   Estimated posterior mean: {mean:.4} ± {std_err:.4}"
                ));
                print_ok(&format!(
                    "[LW]   Effective Sample Size (ESS): {ess:.1} / {n_particles}"
                ));
            }
            Err(e) => print_err(&format!("Failure in Likelihood Weighting: {e}")),
        }

        println!();

        let steps = 4000;
        let warmup = 1000;
        println!("Running SSMH (Steps: {steps}, Warmup: {warmup})...");
        match single_site_mh(model, rng, steps, warmup) {
            Ok(chain) => {
                let (mean, std_err, ess) = mcmc_mean_std_err_ess(&chain);
                print_ok(&format!(
                    "[SSMH] Estimated posterior mean: {mean:.4} ± {std_err:.4}"
                ));
                print_ok(&format!(
                    "[SSMH] Effective Sample Size (ESS, via autocorrelation): {ess:.1} / {steps}"
                ));
            }
            Err(e) => print_err(&format!("Failure in SSMH: {e}")),
        }

        println!(
            "\n   Note: both LW and SSMH combine 'factor' with the same prior, so their\n   estimates should agree with each other and with the analytical value\n   above -- even though neither engine ever sees an explicit 'observe'."
        );
    });
}