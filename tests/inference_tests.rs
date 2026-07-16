/*

Tests for the inference algorithm implementations: lw, smc, ssmh, bbvi and
exact enumeration.

*/

use ppl_tp_final::inference::bbvi::run_bbvi;
use ppl_tp_final::inference::exact_enumeration::*;
use ppl_tp_final::inference::lw::likelihood_weighting;
use ppl_tp_final::inference::smc::run_smc;
use ppl_tp_final::inference::ssmh::single_site_mh;

use rand::rngs::StdRng;
use rand::SeedableRng;

// Conjugate model: Prior Normal(0,1), Observation Normal(mu,1) = 2.3
// Expected analytical posterior mean: 1.15
const CONJUGATE_MODEL: &str = "(let [mu (sample (normal 0 1))] (observe (normal mu 1) 2.3) mu)";
const EXACT_MEAN: f64 = 1.15;
const TOLERANCE: f64 = 0.15; // Allowed tolerance for stochastic algorithms

// Same conjugate model as CONJUGATE_MODEL, but the Normal(mu, 1) log-density
// at 2.3 is computed by hand and pushed with `factor` instead of `observe`.
// The unnormalized log-density (-0.5 * diff^2) differs from the real one by
// a constant (-0.5 * log(2*pi)) that does not depend on `mu`, so it does not
// shift the posterior mean: both models should converge to the same
// EXACT_MEAN. This is exactly what makes these tests a good check that
// `factor` is wired into the log-weight the same way `observe` is.
const FACTOR_MODEL: &str = "(let [mu (sample (normal 0 1)) diff (- mu 2.3) log_lik (* -0.5 (* diff diff))] (factor log_lik) mu)";

#[cfg(test)]
mod inference_algorithms_tests {
    use super::*;

    #[test]
    fn test_likelihood_weighting_convergence() {
        // Fixed seed for reproducibility
        let mut rng = StdRng::seed_from_u64(42);
        let n_particles = 2000;

        let (values, weights) = likelihood_weighting(CONJUGATE_MODEL, n_particles, &mut rng)
            .expect("Failed to execute Likelihood Weighting");

        assert_eq!(values.len(), n_particles);
        assert_eq!(weights.len(), n_particles);

        // 1. Check that the weights sum to ~1.0 (correct softmax normalization)
        let sum_weights: f64 = weights.iter().sum();
        assert!((sum_weights - 1.0).abs() < 1e-6, "Weights do not sum to 1.0");

        // 2. Compute the estimated weighted mean
        let mut estimated_mean = 0.0;
        for (v, w) in values.iter().zip(weights.iter()) {
            let value: f64 = v.as_f64().expect("No numeric value");
            estimated_mean += value * w;
        }

        // 3. Check that it converges to the analytical value
        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "LW estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_sequential_monte_carlo_convergence() {
        let mut rng = StdRng::seed_from_u64(100);
        let n_particles = 1000;

        let results = run_smc(CONJUGATE_MODEL, n_particles, &mut rng)
            .expect("Failed to execute SMC");

        assert_eq!(results.len(), n_particles);

        // In SMC the returned particles are already resampled, so they carry
        // uniform weight. We compute the simple empirical mean.
        let estimated_mean: f64 = results.iter().map(|value| value.as_f64().expect("No numeric value")).sum::<f64>() / (n_particles as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SMC estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_single_site_mh_convergence() {
        let mut rng = StdRng::seed_from_u64(777);
        let steps = 3000;
        let warmup = 1000;

        let (chain, acceptance_rate) = single_site_mh(CONJUGATE_MODEL, &mut rng, steps, warmup)
            .expect("Failed to execute SSMH");

        // The final chain must have exactly length 'steps' (warmup excluded)
        assert_eq!(chain.len(), steps);

        // Compute the mean of the resulting Markov chain
        let estimated_mean: f64 = chain.iter().map(|value| value.as_f64().expect("No numeric value")).sum::<f64>() / (steps as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SSMH estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );

        // CONJUGATE_MODEL has exactly one 'sample' site, so every proposal
        // is a genuine accept/reject attempt -- the rate should be a real
        // number in (0, 1], never NaN.
        assert!(
            acceptance_rate > 0.0 && acceptance_rate <= 1.0,
            "acceptance rate out of the (0, 1] range: {acceptance_rate}"
        );
    }

    #[test]
    fn test_smc_rejects_desynchronized_programs() {
        // In SMC, if the program changes its observe flow inside a condition
        // or a function, it must be rejected by the static analysis BEFORE
        // any particle is instantiated.
        let bad_model = "(if (sample (bernoulli 0.5)) (observe (normal 0 1) 1.0) 0)";
        let mut rng = StdRng::seed_from_u64(123);

        let result = run_smc(bad_model, 50, &mut rng);

        // Check that the static analysis rejects the unsafe model
        assert!(result.is_err(), "SMC should have detected the observe inside the if and returned Err");
        let err_msg = result.unwrap_err();

        // Check that it was caught by the Static Analysis
        assert!(err_msg.contains("Static Analysis Error"), "Incorrect error message: {}", err_msg);
    }


    #[test]
    fn test_bbvi_convergence_coin_flip() {

        // Sample 'x' from a Normal (unbounded range)
        // and compute 'p' using the sigmoid formula: p = 1 / (1 + exp(-x))
        // This way 'p' is always a valid value between 0 and 1 for the Bernoulli.
        let program = r#"
            (let [x (sample (normal 0.0 1.0))
                p (/ 1.0 (+ 1.0 (exp (- 0.0 x))))]
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            (observe (bernoulli p) true)
            p)
        "#;

        let mut rng = StdRng::seed_from_u64(42);

        let (elbo_history, theta_opt, _samples) = run_bbvi(program, 150, 15, 0.05, &mut rng).unwrap();

        let initial_elbo = elbo_history[0];
        let final_elbo = *elbo_history.last().unwrap();

        assert!(
            final_elbo > initial_elbo,
            "The ELBO should increase during optimization. Initial: {}, Final: {}", initial_elbo, final_elbo
        );

        assert!(!theta_opt.is_empty(), "Expected to optimize at least one probabilistic site");
        println!("Initial ELBO: {:.4} -> Final ELBO: {:.4}", initial_elbo, final_elbo);
    }

    #[test]
        fn test_exact_enumeration_8_bit_problem() {
            let bits8 = r#"
            (let [b1 (if (sample (bernoulli 0.5)) 1 0)
                b2 (if (sample (bernoulli 0.5)) 1 0)
                b3 (if (sample (bernoulli 0.5)) 1 0)
                b4 (if (sample (bernoulli 0.5)) 1 0)
                b5 (if (sample (bernoulli 0.5)) 1 0)
                b6 (if (sample (bernoulli 0.5)) 1 0)
                b7 (if (sample (bernoulli 0.5)) 1 0)
                b8 (if (sample (bernoulli 0.5)) 1 0)
                total (+ b1 b2 b3 b4 b5 b6 b7 b8)]
            (observe (normal 7 1) total)
            total)
            "#;

            let runs8 = enumerate_traces(bits8, 10_000).unwrap();
            let (pmf8, log_z8) = posterior_table(&runs8);

            assert_eq!(runs8.len(), 256);
            assert_eq!(pmf8.len(), 9);

            for i in 0..=8 {
                assert!(pmf8.iter().any(|(val, _, _ )| val.as_i64().expect("No numeric value") == i));
            }

            // Relax precision to 1e-8 (equivalent to Python's np.allclose)
            let sum_probs: f64 = pmf8.iter().map(|(_, prob, _)| prob).sum();
            assert!((sum_probs - 1.0).abs() < 1e-8, "Sum of probabilities was: {}", sum_probs);

            let expected_log_z = -2.9387946656298647;
            assert!((log_z8 - expected_log_z).abs() < 1e-8);

            let mean_enum: f64 = pmf8.iter().map(|(val, prob, _)| val.as_i64().expect("No numeric value") as f64 * prob).sum();
            let expected_mean = 6.000655098870;
            assert!((mean_enum - expected_mean).abs() < 1e-8);
        }

    // ---------------------------------------------------------------
    // `factor` operator tests
    //
    // These mirror the corresponding `observe`-based tests above, but
    // replace the observation with a hand-written log-density pushed via
    // `factor`. Since `factor` never pauses the machine (it only touches
    // `Machine::log_w` internally, see `Instr::FactorK` in runtime.rs),
    // each algorithm has to pick that weight up from a different place
    // than it does for `observe`. LW picks it up "for free" because it
    // reads `log_w` straight off the final machine. SMC and SSMH needed an
    // explicit fix to read `log_w` at their respective synchronization/
    // trace-closing points. BBVI and Exact Enumeration were not touched
    // while implementing `factor`, so the two tests below are the actual
    // verification of whether they already support it correctly or not.
    // ---------------------------------------------------------------

    #[test]
    fn test_factor_matches_observe_in_lw() {
        let mut rng = StdRng::seed_from_u64(42);
        let n_particles = 2000;

        let (values, weights) = likelihood_weighting(FACTOR_MODEL, n_particles, &mut rng)
            .expect("Failed to execute Likelihood Weighting with factor");

        let mut estimated_mean = 0.0;
        for (v, w) in values.iter().zip(weights.iter()) {
            let value: f64 = v.as_f64().expect("No numeric value");
            estimated_mean += value * w;
        }

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "LW+factor estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_factor_matches_observe_in_ssmh() {
        // This is the exact scenario that was broken before the ssmh.rs fix:
        // without reading factor_log_w in mh_log_alpha, the chain never
        // moves away from the prior and this assertion fails.
        let mut rng = StdRng::seed_from_u64(777);
        let steps = 3000;
        let warmup = 1000;

        let (chain, acceptance_rate) = single_site_mh(FACTOR_MODEL, &mut rng, steps, warmup)
            .expect("Failed to execute SSMH with factor");

        assert_eq!(chain.len(), steps);

        let estimated_mean: f64 = chain.iter().map(|value| value.as_f64().expect("No numeric value")).sum::<f64>() / (steps as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SSMH+factor estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );

        // FACTOR_MODEL also has exactly one 'sample' site, same reasoning
        // as in test_single_site_mh_convergence above.
        assert!(
            acceptance_rate > 0.0 && acceptance_rate <= 1.0,
            "acceptance rate out of the (0, 1] range: {acceptance_rate}"
        );
    }

    #[test]
    fn test_factor_matches_observe_in_smc() {
        // FACTOR_MODEL has a single synchronization point (the machine only
        // ever pauses on Done, since it has no observe), so this mainly
        // checks that a factor-only model does not desynchronize SMC and
        // that its contribution is not silently dropped before resampling.
        let mut rng = StdRng::seed_from_u64(100);
        let n_particles = 1000;

        let results = run_smc(FACTOR_MODEL, n_particles, &mut rng)
            .expect("Failed to execute SMC with factor");

        assert_eq!(results.len(), n_particles);

        let estimated_mean: f64 = results.iter().map(|value| value.as_f64().expect("No numeric value")).sum::<f64>() / (n_particles as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SMC+factor estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_factor_between_observes_in_smc() {
        // A factor placed between two observe calls, i.e. between two
        // synchronization points. Before the smc.rs fix, log_increments only
        // took the log_prob of the observe at the sync point, silently
        // dropping whatever factor() added on the way there.
        let model = r#"
            (let [p (sample (beta 2.0 2.0))]
                (observe (bernoulli p) true)
                (factor (* 2.0 (- p 0.5)))
                (observe (bernoulli p) true)
                p)
        "#;

        let mut rng = StdRng::seed_from_u64(7);
        let vals = run_smc(model, 2000, &mut rng).expect("Failed to execute SMC with an interleaved factor");
        assert_eq!(vals.len(), 2000);
    }

    #[test]
    fn test_factor_bbvi_does_not_crash_and_optimizes() {
        // Same coin-flip model as test_bbvi_convergence_coin_flip, but the
        // three `observe` calls are collapsed into a single `factor` with
        // the equivalent Bernoulli log-likelihood (log(p) three times, since
        // all three observations are `true`). bbvi.rs was not touched while
        // implementing factor, so this test is the actual check of whether
        // BBVI already reads Machine::log_w correctly or needs a fix
        // analogous to the one applied to ssmh.rs.
        let program = r#"
            (let [x (sample (normal 0.0 1.0))
                p (/ 1.0 (+ 1.0 (exp (- 0.0 x))))]
            (factor (* 3.0 (log p)))
            p)
        "#;

        let mut rng = StdRng::seed_from_u64(42);

        let (elbo_history, theta_opt, _samples) = run_bbvi(program, 150, 15, 0.05, &mut rng)
            .expect("Failed to execute BBVI with factor");

        let initial_elbo = elbo_history[0];
        let final_elbo = *elbo_history.last().unwrap();

        assert!(
            final_elbo > initial_elbo,
            "The ELBO should increase during optimization. Initial: {}, Final: {}", initial_elbo, final_elbo
        );
        assert!(!theta_opt.is_empty(), "Expected to optimize at least one probabilistic site");
    }

    #[test]
    fn test_factor_exact_enumeration_matches_observe() {
        // Same 8-bit model as test_exact_enumeration_8_bit_problem, but
        // (observe (normal 7 1) total) is replaced by a hand-written,
        // *fully normalized* Gaussian log-density pushed via factor
        // (including the -0.5*log(2*pi*sigma^2) term this time, since exact
        // enumeration's log_z is sensitive to normalization constants,
        // unlike the posterior mean checks above). If factor is wired into
        // exact_enumeration.rs the same way observe is, log_z and the
        // posterior mean should match the analytical/observe values
        // exactly (up to floating point error).
        let bits8_factor = r#"
            (let [b1 (if (sample (bernoulli 0.5)) 1 0)
                b2 (if (sample (bernoulli 0.5)) 1 0)
                b3 (if (sample (bernoulli 0.5)) 1 0)
                b4 (if (sample (bernoulli 0.5)) 1 0)
                b5 (if (sample (bernoulli 0.5)) 1 0)
                b6 (if (sample (bernoulli 0.5)) 1 0)
                b7 (if (sample (bernoulli 0.5)) 1 0)
                b8 (if (sample (bernoulli 0.5)) 1 0)
                total (+ b1 b2 b3 b4 b5 b6 b7 b8)
                diff (- total 7)
                log_lik (- (* -0.5 (log 6.283185307179586)) (* 0.5 (* diff diff)))]
            (factor log_lik)
            total)
            "#;

        let runs8 = enumerate_traces(bits8_factor, 10_000)
            .expect("Failed to execute Exact Enumeration with factor");
        let (pmf8, log_z8) = posterior_table(&runs8);

        assert_eq!(runs8.len(), 256);
        assert_eq!(pmf8.len(), 9);

        let expected_log_z = -2.9387946656298647;
        assert!(
            (log_z8 - expected_log_z).abs() < 1e-6,
            "log_z with factor: {}, expected: {}",
            log_z8, expected_log_z
        );

        let mean_enum: f64 = pmf8.iter().map(|(val, prob, _)| val.as_i64().expect("No numeric value") as f64 * prob).sum();
        let expected_mean = 6.000655098870;
        assert!(
            (mean_enum - expected_mean).abs() < 1e-6,
            "Posterior mean with factor: {}, expected: {}",
            mean_enum, expected_mean
        );
    }
}