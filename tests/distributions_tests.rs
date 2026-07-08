// Tests para la implementación de distribuciones y sus operaciones: sample, log_prob, params, with_params, grad_log_prob
use ppl_tp_final::parser::distribution::{make_distribution, make_guide, Distribution};
use ppl_tp_final::parser::value::RVal;
use rand::rngs::ThreadRng;
use approx::assert_relative_eq;

#[cfg(test)]
mod tests_exponential {
    use super::*;

    #[test]
    fn test_exponential_distribution() {
        // Test de validación
        assert!(Distribution::exponential(-1.0).is_err());
        assert!(Distribution::exponential(0.0).is_err());

        // Test de sample
        let dist = Distribution::exponential(1.0).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Float(_)));
    }

    #[test]
    fn test_exponential_log_prob() {
        let dist = Distribution::exponential(1.0).unwrap();

        // rate=1 -> log_prob(0) = ln(1) - 1*0 = 0
        let log_prob_zero = dist.log_prob(&RVal::Float(0.0));
        assert_relative_eq!(log_prob_zero, 0.0, epsilon = 1e-6);

        // rate=1 -> log_prob(1) = ln(1) - 1*1 = -1
        let log_prob_one = dist.log_prob(&RVal::Float(1.0));
        assert_relative_eq!(log_prob_one, -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_exponential_log_prob_outside_support() {
        let dist = Distribution::exponential(2.0).unwrap();
        assert_eq!(dist.log_prob(&RVal::Float(-0.5)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_exponential_params_not_guide() {
        let dist = Distribution::exponential(1.0).unwrap();
        assert!(dist.params().is_none());
        assert!(dist
            .grad_log_prob(&RVal::Float(1.0))
            .is_none());
    }
}

#[cfg(test)]
mod tests_beta {
    use super::*;

    #[test]
    fn test_beta_distribution() {
        // Test de validación
        assert!(Distribution::beta(0.0, 2.0).is_err());
        assert!(Distribution::beta(2.0, -1.0).is_err());

        // Test de sample
        let dist = Distribution::beta(2.0, 2.0).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Float(_)));
    }

    #[test]
    fn test_beta_log_prob() {
        let dist = Distribution::beta(2.0, 2.0).unwrap();
        let log_prob = dist.log_prob(&RVal::Float(0.5));

        // Beta(2,2): log_prob(0.5) = ln(1.5) ≈ 0.4054651
        let expected = 1.5_f64.ln();
        assert_relative_eq!(log_prob, expected, epsilon = 1e-5);
    }

    #[test]
    fn test_beta_log_prob_outside_support() {
        let dist = Distribution::beta(2.0, 2.0).unwrap();
        assert_eq!(dist.log_prob(&RVal::Float(0.0)), f64::NEG_INFINITY);
        assert_eq!(dist.log_prob(&RVal::Float(1.0)), f64::NEG_INFINITY);
        assert_eq!(dist.log_prob(&RVal::Float(1.5)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_beta_params_not_guide() {
        let dist = Distribution::beta(2.0, 2.0).unwrap();
        assert!(dist.params().is_none());
    }
}

#[cfg(test)]
mod tests_gamma {
    use super::*;

    #[test]
    fn test_gamma_distribution() {
        // Test de validación
        assert!(Distribution::gamma(-1.0, 1.0).is_err());
        assert!(Distribution::gamma(1.0, 0.0).is_err());

        // Test de sample
        let dist = Distribution::gamma(2.0, 1.0).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Float(_)));
    }

    #[test]
    fn test_gamma_log_prob() {
        // Gamma(shape=2, rate=1) en x=1: 2*ln(1) - lgamma(2) + 1*ln(1) - 1*1 = -1
        let dist = Distribution::gamma(2.0, 1.0).unwrap();
        let log_prob = dist.log_prob(&RVal::Float(1.0));
        assert_relative_eq!(log_prob, -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_gamma_log_prob_outside_support() {
        let dist = Distribution::gamma(2.0, 1.0).unwrap();
        assert_eq!(dist.log_prob(&RVal::Float(0.0)), f64::NEG_INFINITY);
        assert_eq!(dist.log_prob(&RVal::Float(-1.0)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_gamma_params_not_guide() {
        let dist = Distribution::gamma(2.0, 1.0).unwrap();
        assert!(dist.params().is_none());
    }
}

#[cfg(test)]
mod tests_poisson {
    use super::*;

    #[test]
    fn test_poisson_distribution() {
        // Test de validación
        assert!(Distribution::poisson(0.0).is_err());
        assert!(Distribution::poisson(-3.0).is_err());

        // Test de sample
        let dist = Distribution::poisson(3.0).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Int(_)));
    }

    #[test]
    fn test_poisson_log_prob() {
        let dist = Distribution::poisson(3.0).unwrap();
        let log_prob = dist.log_prob(&RVal::Int(3));

        // k * ln(lam) - lam - lgamma(k+1) = 3*ln(3) - 3 - ln(3!) ≈ -1.4959226
        let expected = -1.4959226;
        assert_relative_eq!(log_prob, expected, epsilon = 1e-5);
    }

    #[test]
    fn test_poisson_log_prob_negative_k() {
        let dist = Distribution::poisson(3.0).unwrap();
        assert_eq!(dist.log_prob(&RVal::Int(-1)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_poisson_incompatible_value() {
        let dist = Distribution::poisson(3.0).unwrap();
        // Poisson espera RVal::Int, no Float
        assert_eq!(
            dist.log_prob(&RVal::Float(3.0)),
            f64::NEG_INFINITY
        );
    }
}

#[cfg(test)]
mod tests_bernoulli {
    use super::*;

    #[test]
    fn test_bernoulli_distribution() {
        // Test de validación
        assert!(Distribution::bernoulli(-0.1).is_err());
        assert!(Distribution::bernoulli(1.1).is_err());

        // Test de sample
        let dist = Distribution::bernoulli(0.5).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Bool(_)));
    }

    #[test]
    fn test_bernoulli_log_prob() {
        let dist = Distribution::bernoulli(0.3).unwrap();

        let log_prob_true = dist.log_prob(&RVal::Bool(true));
        assert_relative_eq!(log_prob_true, 0.3_f64.ln(), epsilon = 1e-6);

        let log_prob_false = dist.log_prob(&RVal::Bool(false));
        assert_relative_eq!(log_prob_false, 0.7_f64.ln(), epsilon = 1e-6);
    }

    #[test]
    fn test_bernoulli_params() {
        let dist = Distribution::bernoulli(0.3).unwrap();
        let params = dist.params().expect("Error distribucion invalida");
        assert_eq!(params.len(), 1);

        // logit(0.3) = ln(0.3/0.7) ≈ -0.8472979
        assert_relative_eq!(params[0], -0.8472979, epsilon = 1e-5);
    }

    #[test]
    fn test_bernoulli_with_params() {
        let dist = Distribution::bernoulli(0.3).unwrap();
        let new_dist = dist.with_params(&[0.0]).expect("Error");

        if let Distribution::Bernoulli { p } = new_dist {
            // sigmoid(0) = 0.5
            assert_relative_eq!(p, 0.5, epsilon = 1e-10);
        } else {
            panic!("with_params no devolvió un Bernoulli");
        }
    }

    #[test]
    fn test_bernoulli_grad_log_prob() {
        let dist = Distribution::bernoulli(0.3).unwrap();

        let grad_true = dist.grad_log_prob(&RVal::Bool(true)).unwrap();
        assert_relative_eq!(grad_true[0], 0.7, epsilon = 1e-6);

        let grad_false = dist.grad_log_prob(&RVal::Bool(false)).unwrap();
        assert_relative_eq!(grad_false[0], -0.3, epsilon = 1e-6);
    }

    #[test]
    fn test_bernoulli_incompatible_value() {
        let dist = Distribution::bernoulli(0.3).unwrap();
        let wrong_value = RVal::Float(1.0);
        assert!(dist.grad_log_prob(&wrong_value).is_none());
    }
}

#[cfg(test)]
mod tests_discrete {
    use super::*;

    #[test]
    fn test_discrete_distribution() {
        // Test de validación
        assert!(Distribution::discrete(&[-0.5, 0.5]).is_err());
        assert!(Distribution::discrete(&[0.0, 0.0]).is_err());

        // Test de sample
        let dist = Distribution::discrete(&[0.2, 0.3, 0.5]).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Int(_)));
    }

    #[test]
    fn test_discrete_normalizes_probs() {
        // No normalizado: [2, 2] -> debería quedar [0.5, 0.5]
        let dist = Distribution::discrete(&[2.0, 2.0]).unwrap();
        if let Distribution::Discrete { probs } = dist {
            assert_relative_eq!(probs[0], 0.5, epsilon = 1e-10);
            assert_relative_eq!(probs[1], 0.5, epsilon = 1e-10);
        } else {
            panic!("no se construyó un Discrete");
        }
    }

    #[test]
    fn test_discrete_log_prob() {
        let dist = Distribution::discrete(&[0.2, 0.3, 0.5]).unwrap();
        let log_prob = dist.log_prob(&RVal::Int(2));
        assert_relative_eq!(log_prob, 0.5_f64.ln(), epsilon = 1e-6);
    }

    #[test]
    fn test_discrete_log_prob_out_of_range() {
        let dist = Distribution::discrete(&[0.2, 0.3, 0.5]).unwrap();
        assert_eq!(dist.log_prob(&RVal::Int(-1)), f64::NEG_INFINITY);
        assert_eq!(dist.log_prob(&RVal::Int(3)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_discrete_params() {
        let dist = Distribution::discrete(&[0.5, 0.5]).unwrap();
        let params = dist.params().expect("Error distribucion invalida");
        assert_eq!(params.len(), 2);
        assert_relative_eq!(params[0], 0.5_f64.ln(), epsilon = 1e-6);
    }

    #[test]
    fn test_discrete_with_params() {
        let dist = Distribution::discrete(&[0.5, 0.5]).unwrap();
        let new_dist = dist.with_params(&[0.0, 0.0]).expect("Error");

        if let Distribution::Discrete { probs } = new_dist {
            // softmax([0,0]) = [0.5, 0.5]
            assert_relative_eq!(probs[0], 0.5, epsilon = 1e-10);
            assert_relative_eq!(probs[1], 0.5, epsilon = 1e-10);
        } else {
            panic!("with_params no devolvió un Discrete");
        }
    }

    #[test]
    fn test_discrete_grad_log_prob() {
        let dist = Distribution::discrete(&[0.5, 0.5]).unwrap();
        let grad = dist.grad_log_prob(&RVal::Int(0)).unwrap();

        assert_relative_eq!(grad[0], 0.5, epsilon = 1e-6);
        assert_relative_eq!(grad[1], -0.5, epsilon = 1e-6);
    }

    #[test]
    fn test_discrete_incompatible_value() {
        let dist = Distribution::discrete(&[0.5, 0.5]).unwrap();
        let wrong_value = RVal::Float(0.0);
        assert!(dist.grad_log_prob(&wrong_value).is_none());
    }
}

#[cfg(test)]
mod tests_uniform_discrete {
    use super::*;

    #[test]
    fn test_uniform_discrete_distribution() {
        // Test de validación
        assert!(Distribution::uniform_discrete(5, 5).is_err());
        assert!(Distribution::uniform_discrete(5, 2).is_err());

        // Test de sample
        let dist = Distribution::uniform_discrete(0, 5).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::Int(_)));

        if let RVal::Int(k) = sample {
            assert!((0..5).contains(&k));
        }
    }

    #[test]
    fn test_uniform_discrete_log_prob() {
        let dist = Distribution::uniform_discrete(0, 5).unwrap();
        let log_prob = dist.log_prob(&RVal::Int(2));
        assert_relative_eq!(log_prob, -(5.0_f64.ln()), epsilon = 1e-6);
    }

    #[test]
    fn test_uniform_discrete_log_prob_outside_support() {
        let dist = Distribution::uniform_discrete(0, 5).unwrap();
        assert_eq!(dist.log_prob(&RVal::Int(-1)), f64::NEG_INFINITY);
        assert_eq!(dist.log_prob(&RVal::Int(5)), f64::NEG_INFINITY);
    }

    #[test]
    fn test_uniform_discrete_params_not_guide() {
        let dist = Distribution::uniform_discrete(0, 5).unwrap();
        assert!(dist.params().is_none());
    }
}

#[cfg(test)]
mod tests_dirichlet {
    use super::*;

    #[test]
    fn test_dirichlet_distribution() {
        // Test de validación
        assert!(Distribution::dirichlet(&[1.0, 0.0, 1.0]).is_err());
        assert!(Distribution::dirichlet(&[1.0, -1.0]).is_err());

        // Test de sample
        let dist = Distribution::dirichlet(&[1.0, 1.0, 1.0]).unwrap();
        let mut rng = ThreadRng::default();
        let sample = dist.sample(&mut rng);

        assert!(matches!(sample, RVal::List(_)));

        if let RVal::List(v) = sample {
            assert_eq!(v.len(), 3);
            let sum: f64 = v.iter().map(|elem|  match  elem  {
                RVal::Float(f) => *f,
                _ => 0.0,
            }).sum();
            assert_relative_eq!(sum, 1.0, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_dirichlet_log_prob() {
        // Dirichlet(1,1,1) es uniforme en el simplex 2-D, densidad constante = 1/B(1,1,1) = 2
        let dist = Distribution::dirichlet(&[1.0, 1.0, 1.0]).unwrap();
        let log_prob = dist.log_prob(&RVal::List(vec![RVal::Float(1.0/3.0), RVal::Float(1.0/3.0), RVal::Float(1.0/3.0)]));
        assert_relative_eq!(log_prob, 2.0_f64.ln(), epsilon = 1e-5);
    }

    #[test]
    fn test_dirichlet_log_prob_wrong_length() {
        let dist = Distribution::dirichlet(&[1.0, 1.0, 1.0]).unwrap();
        let log_prob = dist.log_prob(&RVal::List(vec![RVal::Float(0.5), RVal::Float(0.5)]));
        assert_eq!(log_prob, f64::NEG_INFINITY);
    }

    #[test]
    fn test_dirichlet_log_prob_outside_support() {
        let dist = Distribution::dirichlet(&[1.0, 1.0, 1.0]).unwrap();
        let log_prob = dist.log_prob(&RVal::List(vec![RVal::Float(0.0), RVal::Float(0.5), RVal::Float(0.5)]));
        assert_eq!(log_prob, f64::NEG_INFINITY);
    }

    #[test]
    fn test_dirichlet_params_not_guide() {
        let dist = Distribution::dirichlet(&[1.0, 1.0, 1.0]).unwrap();
        assert!(dist.params().is_none());
    }
}

#[cfg(test)]
mod tests_make_distribution {
    use super::*;

    #[test]
    fn test_make_distribution_normal() {
        let dist = make_distribution("normal", &[0.0, 1.0]).unwrap();
        assert!(matches!(dist, Distribution::Normal { .. }));
    }

    #[test]
    fn test_make_distribution_aliases() {
        // "uniform" y "uniform-continuous" deben construir lo mismo
        let d1 = make_distribution("uniform", &[0.0, 1.0]).unwrap();
        let d2 = make_distribution("uniform-continuous", &[0.0, 1.0]).unwrap();
        assert!(matches!(d1, Distribution::Uniform { .. }));
        assert!(matches!(d2, Distribution::Uniform { .. }));

        // "bernoulli" y "flip" deben construir lo mismo
        let d3 = make_distribution("bernoulli", &[0.5]).unwrap();
        let d4 = make_distribution("flip", &[0.5]).unwrap();
        assert!(matches!(d3, Distribution::Bernoulli { .. }));
        assert!(matches!(d4, Distribution::Bernoulli { .. }));

        // "discrete" y "categorical" deben construir lo mismo
        let d5 = make_distribution("discrete", &[0.5, 0.5]).unwrap();
        let d6 = make_distribution("categorical", &[0.5, 0.5]).unwrap();
        assert!(matches!(d5, Distribution::Discrete { .. }));
        assert!(matches!(d6, Distribution::Discrete { .. }));
    }

    #[test]
    fn test_make_distribution_variable_arity() {
        // dirichlet con K=3 alphas
        let dist = make_distribution("dirichlet", &[1.0, 1.0, 1.0]).unwrap();
        if let Distribution::Dirichlet { alphas } = dist {
            assert_eq!(alphas.len(), 3);
        } else {
            panic!("no se construyó un Dirichlet");
        }
    }

    #[test]
    fn test_make_distribution_uniform_discrete() {
        let dist = make_distribution("uniform-discrete", &[0.0, 5.0]).unwrap();
        assert!(matches!(dist, Distribution::UniformDiscrete { lo: 0, hi: 5 }));
    }

    #[test]
    fn test_make_distribution_unknown_name() {
        let result = make_distribution("no-existe", &[1.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_make_distribution_invalid_params() {
        // sigma negativo -> Err propagado desde Distribution::normal
        let result = make_distribution("normal", &[0.0, -1.0]);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests_make_guide {
    use super::*;

    #[test]
    fn test_make_guide_normal_identity() {
        let d = Distribution::normal(1.0, 2.0).unwrap();
        let guide = make_guide(&d).unwrap();
        assert!(matches!(guide, Distribution::Normal { .. }));
    }

    #[test]
    fn test_make_guide_positive_support_to_log_normal() {
        let gamma = Distribution::gamma(2.0, 1.0).unwrap();
        let exponential = Distribution::exponential(1.0).unwrap();

        for d in [gamma, exponential] {
            let guide = make_guide(&d).unwrap();
            if let Distribution::LogNormal { mu, sigma } = guide {
                assert_relative_eq!(mu, 0.0, epsilon = 1e-10);
                assert_relative_eq!(sigma, 1.0, epsilon = 1e-10);
            } else {
                panic!("guide debería ser LogNormal(0,1)");
            }
        }
    }

    #[test]
    fn test_make_guide_bernoulli_identity() {
        let d = Distribution::bernoulli(0.3).unwrap();
        let guide = make_guide(&d).unwrap();
        if let Distribution::Bernoulli { p } = guide {
            assert_relative_eq!(p, 0.3, epsilon = 1e-10);
        } else {
            panic!("guide debería ser Bernoulli");
        }
    }

    #[test]
    fn test_make_guide_discrete_identity() {
        let d = Distribution::discrete(&[0.2, 0.3, 0.5]).unwrap();
        let guide = make_guide(&d).unwrap();
        assert!(matches!(guide, Distribution::Discrete { .. }));
    }

    #[test]
    fn test_make_guide_unsupported_family() {
        // Uniform, Poisson, Dirichlet y UniformDiscrete no tienen familia de guide
        let uniform = Distribution::uniform(0.0, 1.0).unwrap();
        let poisson = Distribution::poisson(3.0).unwrap();
        let dirichlet = Distribution::dirichlet(&[1.0, 1.0]).unwrap();
        let uniform_discrete = Distribution::uniform_discrete(0, 5).unwrap();

        assert!(make_guide(&uniform).is_err());
        assert!(make_guide(&poisson).is_err());
        assert!(make_guide(&dirichlet).is_err());
        assert!(make_guide(&uniform_discrete).is_err());
    }
} 