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
        _ => panic!("Se esperaba un valor numérico"),
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
            .expect("Fallo al ejecutar Likelihood Weighting");

        assert_eq!(values.len(), n_particles);
        assert_eq!(weights.len(), n_particles);

        // 1. Verificamos que los pesos sumen ~1.0 (Normalización Softmax correcta)
        let sum_weights: f64 = weights.iter().sum();
        assert!((sum_weights - 1.0).abs() < 1e-6, "Los pesos no suman 1.0");

        // 2. Calculamos la media ponderada estimada
        let mut estimated_mean = 0.0;
        for (v, w) in values.iter().zip(weights.iter()) {
            estimated_mean += as_f64(v) * w;
        }

        // 3. Verificamos que converja al valor analítico
        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "LW media estimada: {}, esperada: {}. Error ({}) supera la tolerancia.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_sequential_monte_carlo_convergence() {
        let mut rng = StdRng::seed_from_u64(100);
        let n_particles = 1000;

        let results = run_smc(CONJUGATE_MODEL, n_particles, &mut rng)
            .expect("Fallo al ejecutar SMC");

        assert_eq!(results.len(), n_particles);

        // En SMC las partículas devueltas ya están re-muestreadas, por lo que 
        // tienen peso uniforme. Calculamos la media empírica simple.
        let estimated_mean: f64 = results.iter().map(as_f64).sum::<f64>() / (n_particles as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SMC media estimada: {}, esperada: {}. Error ({}) supera la tolerancia.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_single_site_mh_convergence() {
        let mut rng = StdRng::seed_from_u64(777);
        let steps = 3000;
        let warmup = 1000;

        let chain = single_site_mh(CONJUGATE_MODEL, &mut rng, steps, warmup)
            .expect("Fallo al ejecutar SSMH");

        // La cadena final debe tener la longitud exacta de 'steps' (sin incluir warmup)
        assert_eq!(chain.len(), steps);

        // Calculamos la media de la cadena de Markov resultante
        let estimated_mean: f64 = chain.iter().map(as_f64).sum::<f64>() / (steps as f64);

        let error = (estimated_mean - EXACT_MEAN).abs();
        assert!(
            error < TOLERANCE,
            "SSMH media estimada: {}, esperada: {}. Error ({}) supera la tolerancia.",
            estimated_mean, EXACT_MEAN, error
        );
    }

    #[test]
    fn test_smc_rejects_desynchronized_programs() {
        // En SMC, si el programa cambia su flujo de observe dependiendo de algo aleatorio, debe fallar.
        // Aquí forzamos una desincronización: la mitad de las máquinas harán observe, la otra mitad no.
        // TODO: La verificacion es dinamica, despues agrego una verificacion estatica.
        let bad_model = "(if (sample (bernoulli 0.5)) (observe (normal 0 1) 1.0) 0)";
        let mut rng = StdRng::seed_from_u64(123);
        
        let result = run_smc(bad_model, 50, &mut rng);
        
        // Verificamos que falle y atrape la desincronización
        assert!(result.is_err(), "SMC debió detectar la desincronización y retornar Err");
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Desynchronization"), "Mensaje de error incorrecto: {}", err_msg);
    }
}