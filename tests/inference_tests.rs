use PPL_TP_FINAL::inference::lw::likelihood_weighting;
use PPL_TP_FINAL::inference::smc::run_smc;
use PPL_TP_FINAL::inference::ssmh::single_site_mh;
use PPL_TP_FINAL::parser::value::RVal;

use rand::rngs::StdRng;
use rand::SeedableRng;

// Modelo conjugado: Prior Normal(0,1), Observación Normal(mu,1) = 2.3
// Media analítica esperada del posterior: 1.15
const CONJUGATE_MODEL: &str = "(let [mu (sample (normal 0 1))] (observe (normal mu 1) 2.3) mu)";
const EXACT_MEAN: f64 = 1.15;
const TOLERANCE: f64 = 0.15; // Tolerancia permitida para los algoritmos estocásticos

/// Función auxiliar para extraer el valor f64 de un RVal
fn as_f64(val: &RVal) -> f64 {
    match val {
        RVal::Float(f) => *f,
        RVal::Int(i) => *i as f64,
        _ => panic!("Expected a numeric value"),
    }
}

#[cfg(test)]
mod inference_algorithms_tests {
    use super::*;

    #[test]
    fn test_likelihood_weighting_convergence() {
        // Semilla fija para reproducibilidad
        let mut rng = StdRng::seed_from_u64(42);
        let n_particles = 2000;

        let (values, weights) = likelihood_weighting(CONJUGATE_MODEL, n_particles, &mut rng)
            .expect("Failed to execute Likelihood Weighting");

        assert_eq!(values.len(), n_particles);
        assert_eq!(weights.len(), n_particles);

        // 1. Verificamos que los pesos sumen ~1.0 (Normalización Softmax correcta)
        let sum_weights: f64 = weights.iter().sum();
        assert!((sum_weights - 1.0).abs() < 1e-6, "Weights do not sum to 1.0");

        // 2. Calculamos la media ponderada estimada
        let mut estimated_mean = 0.0;
        for (v, w) in values.iter().zip(weights.iter()) {
            estimated_mean += as_f64(v) * w;
        }

        // 3. Verificamos que converja al valor analítico
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

        // En SMC las partículas devueltas ya están re-muestreadas, por lo que 
        // tienen peso uniforme. Calculamos la media empírica simple.
        let estimated_mean: f64 = results.iter().map(as_f64).sum::<f64>() / (n_particles as f64);

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

        let chain = single_site_mh(CONJUGATE_MODEL, &mut rng, steps, warmup)
            .expect("Failed to execute SSMH");

        // La cadena final debe tener la longitud exacta de 'steps' (sin incluir warmup)
        assert_eq!(chain.len(), steps);

        // Calculamos la media de la cadena de Markov resultante
        let estimated_mean: f64 = chain.iter().map(as_f64).sum::<f64>() / (steps as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SSMH estimated mean: {}, expected: {}. Error ({}) exceeds tolerance.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_smc_rejects_desynchronized_programs() {
        // En SMC, si el programa cambia su flujo de observe dentro de una condición o función,
        // debe ser rechazado por el análisis estático ANTES de instanciar las partículas.
        let bad_model = "(if (sample (bernoulli 0.5)) (observe (normal 0 1) 1.0) 0)";
        let mut rng = StdRng::seed_from_u64(123);
        
        let result = run_smc(bad_model, 50, &mut rng);
        
        // Verificamos que el análisis estático rechaze el modelo inseguro
        assert!(result.is_err(), "SMC should have detected the observe inside the if and returned Err");
        let err_msg = result.unwrap_err();
        
        // Verificamos que haya sido atrapado por el Static Analysis
        assert!(err_msg.contains("Static Analysis Error"), "Incorrect error message: {}", err_msg);
    }

    #[test]
#[test]
fn test_bbvi_convergence_coin_flip() {
    use PPL_TP_FINAL::inference::bbvi::run_bbvi;
    use rand::prelude::*;

    // Muestreamos 'x' de una Normal (rango ilimitado) 
    // y calculamos 'p' usando la fórmula de la sigmoide: p = 1 / (1 + exp(-x))
    // De esta forma, 'p' siempre será un valor válido entre 0 y 1 para el Bernoulli.
    let program = r#"
        (let [x (sample (normal 0.0 1.0))
              p (/ 1.0 (+ 1.0 (exp (- 0.0 x))))]
          (observe (bernoulli p) true)
          (observe (bernoulli p) true)
          (observe (bernoulli p) true)
          p)
    "#;

    let mut rng = StdRng::seed_from_u64(42);
    
    let (elbo_history, theta_opt) = run_bbvi(program, 150, 15, 0.05, &mut rng).unwrap();

    let initial_elbo = elbo_history[0];
    let final_elbo = *elbo_history.last().unwrap();
    
    assert!(
        final_elbo > initial_elbo,
        "The ELBO should increase during optimization. Initial: {}, Final: {}", initial_elbo, final_elbo
    );

    assert!(!theta_opt.is_empty(), "Expected to optimize at least one probabilistic site");
    println!("Initial ELBO: {:.4} -> Final ELBO: {:.4}", initial_elbo, final_elbo);
}
}