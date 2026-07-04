// Ahora vamos a testear las primitivas, para eso vamos a crear un archivo de test en tests/primitives_tests.rs
use PPL_TP_FINAL::parser::primitives::{make_primitives, is_primitive, HashKey};
use PPL_TP_FINAL::parser::value::RVal;
use PPL_TP_FINAL::parser::distribution::{Distribution};
use std::collections::HashMap;

#[cfg(test)]
mod primitives_arithmetic_tests {
    // Tests de funciones aritmeticas basicas.
    use super::*;
    #[test]
    fn test_is_primitive() {
        assert!(is_primitive("+"));
        assert!(is_primitive("-"));
        assert!(is_primitive("flip"));
        assert!(!is_primitive("nonexistent"));
    }

    #[test]
    fn test_make_primitives() {
        let primitives = make_primitives();
        assert!(primitives.contains_key("+"));
        assert!(primitives.contains_key("-"));
        assert!(!primitives.contains_key("nonexistent"));
    }

    #[test]
    fn test_prim_add() {
        let args = vec![RVal::Int(5), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("+").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Int(8));
    }

    #[test]
    fn test_prim_sub() {
        let args = vec![RVal::Int(5), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("-").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Int(2));
    }

    #[test]
    fn test_prim_mul() {
        let args = vec![RVal::Int(5), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("*").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Int(15));
    }

    #[test]
    fn test_prim_div() {
        let args = vec![RVal::Int(10), RVal::Int(2)];
        let primitives = make_primitives();
        let function = primitives.get("/").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Int(5));
    }

    #[test]
    fn test_division_by_zero() {
        let args = vec![RVal::Int(5), RVal::Int(0)];
        let primitives = make_primitives();
        let function = primitives.get("/").unwrap();
        let result = function(&args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Arithmetic error: division by zero");
    }

    #[test]
    fn test_prim_mod() {
        let args = vec![RVal::Int(10), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("mod").unwrap();
        let result = function(&args);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Int(1));
    }   

    #[test]
    fn test_modulo_by_zero() {
        let args = vec![RVal::Int(5), RVal::Int(0)];
        let primitives = make_primitives();
        let function = primitives.get("mod").unwrap();
        let result = function(&args);
        print!("Result: {:?}", result);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Arithmetic error: modulo by zero");
    }

    #[test]
    fn test_prim_lt() {
        let args = vec![RVal::Int(5), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("<").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(false));
    }

    #[test]
    fn test_prim_gt() {
        let args = vec![RVal::Int(5), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get(">").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(true));
    }

    #[test]
    fn test_prim_eq() {
        let args = vec![RVal::Int(5), RVal::Int(5)];
        let primitives = make_primitives();
        let function = primitives.get("=").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(true));
    }

    #[test]
    fn test_prim_not() {
        let args = vec![RVal::Bool(true)];
        let primitives = make_primitives();
        let function = primitives.get("not").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(false));
    }

    #[test]
    fn test_prim_and() {
        let args = vec![RVal::Bool(true), RVal::Bool(false)];
        let primitives = make_primitives();
        let function = primitives.get("and").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(false));
    }   

    #[test]
    fn test_prim_or() {
        let args = vec![RVal::Bool(true), RVal::Bool(false)];
        let primitives = make_primitives();
        let function = primitives.get("or").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(true));
    }

    #[test]
    fn test_prim_lte() {
        let args = vec![RVal::Int(5), RVal::Int(5)];
        let primitives = make_primitives();
        let function = primitives.get("<=").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(true));
    }

    #[test]
    fn test_prim_gte() {
        let args = vec![RVal::Int(5), RVal::Int(5)];
        let primitives = make_primitives();
        let function = primitives.get(">=").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::Bool(true));
    }
    // Agregar mas en el futuro
}

#[cfg(test)]
mod primitives_distribution_tests {
    use super::*;

    #[test]
    fn test_distribution_bernoulli_primitives() {
    let primitives = make_primitives();
    let flip_function = primitives.get("flip").unwrap();
    let args = vec![RVal::Float(0.5)];
    let result = flip_function(&args);
    
    assert!(result.is_ok());
    
    match result.unwrap() {
        RVal::Dist(dist) => {
            // 2. Desestructuramos el enum Distribution para acceder a 'p'
            if let Distribution::Bernoulli { p } = dist {
                assert_eq!(dist.name(), "flip");
                assert_eq!(p, 0.5);
            } else {
                panic!("Expected Bernoulli distribution variant");
            }
        }
        _ => panic!("Expected RVal::Dist"),
    }
}

    #[test]
    fn test_distribution_uniform_primitives() {
        let primitives = make_primitives();
        let uniform_function = primitives.get("uniform").unwrap();
        let args = vec![RVal::Float(0.0), RVal::Float(1.0)];
        let result = uniform_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Uniform { a, b } = dist {
                    assert_eq!(dist.name(), "uniform-continuous");
                    assert_eq!(a, 0.0);
                    assert_eq!(b, 1.0);
                } else {
                    panic!("Expected Uniform distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_exponential_primitives() {
        let primitives = make_primitives();
        let exponential_function = primitives.get("exponential").unwrap();
        let args = vec![RVal::Float(1.0)];
        let result = exponential_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Exponential { rate } = dist {
                    assert_eq!(dist.name(), "exponential");
                    assert_eq!(rate, 1.0);
                } else {
                    panic!("Expected Exponential distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_normal_primitives() {
        let primitives = make_primitives();
        let normal_function = primitives.get("normal").unwrap();
        let args = vec![RVal::Float(0.0), RVal::Float(1.0)];
        let result = normal_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Normal { mu, sigma } = dist {
                    assert_eq!(dist.name(), "normal");
                    assert_eq!(mu, 0.0);
                    assert_eq!(sigma, 1.0);
                } else {
                    panic!("Expected Normal distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_gamma_primitives() {
        let primitives = make_primitives();
        let gamma_function = primitives.get("gamma").unwrap();
        let args = vec![RVal::Float(1.0), RVal::Float(1.0)];
        let result = gamma_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Gamma { shape, rate } = dist {
                    assert_eq!(dist.name(), "gamma");
                    assert_eq!(shape, 1.0);
                    assert_eq!(rate, 1.0);
                } else {
                    panic!("Expected Gamma distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_beta_primitives() {
        let primitives = make_primitives();
        let beta_function = primitives.get("beta").unwrap();
        let args = vec![RVal::Float(1.0), RVal::Float(1.0)];
        let result = beta_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Beta { alpha, beta } = dist {
                    assert_eq!(dist.name(), "beta");
                    assert_eq!(alpha, 1.0);
                    assert_eq!(beta, 1.0);
                } else {
                    panic!("Expected Beta distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_lognormal_primitives() {
        let primitives = make_primitives();
        let lognormal_function = primitives.get("log-normal").unwrap();
        let args = vec![RVal::Float(0.0), RVal::Float(1.0)];
        let result = lognormal_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::LogNormal { mu, sigma } = dist {
                    assert_eq!(dist.name(), "log-normal");
                    assert_eq!(mu, 0.0);
                    assert_eq!(sigma, 1.0);
                } else {
                    panic!("Expected LogNormal distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_poisson_primitives() {
        let primitives = make_primitives();
        let poisson_function = primitives.get("poisson").unwrap();
        let args = vec![RVal::Float(1.0)];
        let result = poisson_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Poisson { lam } = dist {
                    assert_eq!(dist.name(), "poisson");
                    assert_eq!(lam, 1.0);
                } else {
                    panic!("Expected Poisson distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_discrete_primitives() {
        let primitives = make_primitives();
        let discrete_function = primitives.get("discrete").unwrap();
        let args = vec![RVal::List(vec![RVal::Float(0.2), RVal::Float(0.5), RVal::Float(0.3)])];
        let result = discrete_function(&args);
        
        print!("Result: {:?}", result);
        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Discrete { probs } = &dist {
                    assert_eq!(dist.name(), "discrete");
                    assert_eq!(probs, &vec![0.2, 0.5, 0.3]);
                } else {
                    panic!("Expected Discrete distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    // Mismo test solo que con otro alias para la distribución categórica, que es "discrete"
    #[test]
    fn test_distribution_categorical_primitives() {
        let primitives = make_primitives();
        let categorical_function = primitives.get("discrete").unwrap();
        let args = vec![RVal::List(vec![RVal::Float(0.2), RVal::Float(0.5), RVal::Float(0.3)])];
        let result = categorical_function(&args);
        
        print!("Result: {:?}", result);
        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Discrete { probs } = &dist {
                    assert_eq!(dist.name(), "discrete");
                    assert_eq!(probs, &vec![0.2, 0.5, 0.3]);
                } else {
                    panic!("Expected Categorical distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }

    #[test]
    fn test_distribution_dirichlet_primitives() {
        let primitives = make_primitives();
        let dirichlet_function = primitives.get("dirichlet").unwrap();
        let args = vec![RVal::List(vec![RVal::Float(1.0), RVal::Float(1.0), RVal::Float(1.0)])];
        let result = dirichlet_function(&args);

        assert!(result.is_ok());

        match result.unwrap() {
            RVal::Dist(dist) => {
                if let Distribution::Dirichlet { alphas } = &dist {
                    assert_eq!(dist.name(), "dirichlet");
                    assert_eq!(alphas, &vec![1.0, 1.0, 1.0]);
                } else {
                    panic!("Expected Dirichlet distribution variant");
                }
            }
            _ => panic!("Expected RVal::Dist"),
        }
    }
    
}

#[cfg(test)]
mod tests_data_structures_operations {
    use super::*;
    
    // Tests de operaciones con estructuras de datos.
    #[test]
    fn test_make_vector() {
        let args = vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("vector").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::List(vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)]));
    }

    #[test]
    fn test_make_list() {
        let args = vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)];
        let primitives = make_primitives();
        let function = primitives.get("list").unwrap();
        let result = function(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RVal::List(vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)]));
    }

    #[test]
    fn test_make_hashmap() {
        let args = vec![
            RVal::Str("a".to_string()), RVal::Int(1),
            RVal::Str("b".to_string()), RVal::Int(2),
        ];
        let primitives = make_primitives();
        let function = primitives.get("hash-map").unwrap();
        let result = function(&args).unwrap();
        
        if let RVal::Map(m) = result {
            assert_eq!(m.len(), 2);
            assert_eq!(m.get(&HashKey::Str("a".to_string())), Some(&RVal::Int(1)));
            assert_eq!(m.get(&HashKey::Str("b".to_string())), Some(&RVal::Int(2)));
        } else {
            panic!("El resultado no es un RVal::Map");
        }
    }

    #[test]
    fn test_get_operation() {
        let primitives = make_primitives();
        let get_fn = primitives.get("get").unwrap();

        // Caso Map
        let m = RVal::Map(vec![
            (HashKey::Str("a".to_string()), RVal::Int(10))
        ].into_iter().collect());
        
        let res_map = get_fn(&[m, RVal::Str("a".to_string())]).unwrap();
        assert_eq!(res_map, RVal::Int(10));

        // Caso List
        let l = RVal::List(vec![RVal::Int(100), RVal::Int(200)]);
        let res_list = get_fn(&[l, RVal::Int(1)]).unwrap();
        assert_eq!(res_list, RVal::Int(200));
    }

    #[test]
    fn test_put_operation() {
        let primitives = make_primitives();
        let put_fn = primitives.get("put").unwrap();

        // Caso Map (actualización de clave)
        let m = RVal::Map(vec![
            (HashKey::Str("x".to_string()), RVal::Int(1))
        ].into_iter().collect());
        
        let res_map = put_fn(&[m, RVal::Str("x".to_string()), RVal::Int(5)]).unwrap();
        if let RVal::Map(new_m) = res_map {
            assert_eq!(new_m.get(&HashKey::Str("x".to_string())), Some(&RVal::Int(5)));
        } else {
            panic!("Expected RVal::Map");
        }

        // Caso List (actualización de índice)
        let l = RVal::List(vec![RVal::Int(0), RVal::Int(10)]);
        let res_list = put_fn(&[l, RVal::Int(0), RVal::Int(99)]).unwrap();
        assert_eq!(res_list, RVal::List(vec![RVal::Int(99), RVal::Int(10)]));
    }

    #[test]
    fn test_first_operation() {
        let primitives = make_primitives();
        let first_fn = primitives.get("first").unwrap();

        // Caso exitoso
        let l = RVal::List(vec![RVal::Int(10), RVal::Int(20)]);
        let res = first_fn(&[l]).unwrap();
        assert_eq!(res, RVal::Int(10));

        // Caso error: lista vacía
        let empty_l = RVal::List(vec![]);
        let res_err = first_fn(&[empty_l]);
        assert!(res_err.is_err());
        assert_eq!(res_err.unwrap_err(), "Value error in 'first': cannot get the first element of an empty List");
    }

    #[test]
    fn test_second_operation() {
        let primitives = make_primitives();
        let second_fn = primitives.get("second").unwrap();

        // Caso exitoso
        let l = RVal::List(vec![RVal::Int(10), RVal::Int(20), RVal::Int(30)]);
        let res = second_fn(&[l]).unwrap();
        assert_eq!(res, RVal::Int(20));

        // Caso error: lista muy corta
        let short_l = RVal::List(vec![RVal::Int(10)]);
        let res_err = second_fn(&[short_l]);
        assert!(res_err.is_err());
        assert_eq!(res_err.unwrap_err(), "Value error in 'second': the List does not have a second element");
    }

    #[test]
    fn test_last_operation() {
        let primitives = make_primitives();
        let last_fn = primitives.get("last").unwrap();

        // Caso exitoso
        let l = RVal::List(vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)]);
        let res = last_fn(&[l]).unwrap();
        assert_eq!(res, RVal::Int(3));

        // Caso error: lista vacía
        let empty_l = RVal::List(vec![]);
        let res_err = last_fn(&[empty_l]);
        assert!(res_err.is_err());
        assert_eq!(res_err.unwrap_err(), "Value error in 'last': cannot get the last element of an empty List");
    }

    #[test]
    fn test_rest_operation() {
        let primitives = make_primitives();
        let rest_fn = primitives.get("rest").unwrap();

        // Caso exitoso
        let l = RVal::List(vec![RVal::Int(10), RVal::Int(20), RVal::Int(30)]);
        let res = rest_fn(&[l]).unwrap();
        assert_eq!(res, RVal::List(vec![RVal::Int(20), RVal::Int(30)]));

        // Caso límite: lista con un elemento
        let single_l = RVal::List(vec![RVal::Int(10)]);
        let res_single = rest_fn(&[single_l]).unwrap();
        assert_eq!(res_single, RVal::List(vec![]));
    }

    #[test]
    fn test_collection_operations() {
        let primitives = make_primitives();
        
        // Test conj: añade al final
        let conj_fn = primitives.get("conj").unwrap();
        let res_conj = conj_fn(&[RVal::List(vec![RVal::Int(1)]), RVal::Int(2)]).unwrap();
        assert_eq!(res_conj, RVal::List(vec![RVal::Int(1), RVal::Int(2)]));

        // Test cons: añade al principio
        let cons_fn = primitives.get("cons").unwrap();
        let res_cons = cons_fn(&[RVal::Int(0), RVal::List(vec![RVal::Int(1)])]).unwrap();
        assert_eq!(res_cons, RVal::List(vec![RVal::Int(0), RVal::Int(1)]));

        // Test append: concatena al final
        let append_fn = primitives.get("append").unwrap();
        let res_app = append_fn(&[RVal::List(vec![RVal::Int(1)]), RVal::Int(2), RVal::Int(3)]).unwrap();
        assert_eq!(res_app, RVal::List(vec![RVal::Int(1), RVal::Int(2), RVal::Int(3)]));

        // Test concat: une listas
        let concat_fn = primitives.get("concat").unwrap();
        let res_cat = concat_fn(&[RVal::List(vec![RVal::Int(1)]), RVal::List(vec![RVal::Int(2)])]).unwrap();
        assert_eq!(res_cat, RVal::List(vec![RVal::Int(1), RVal::Int(2)]));
    }

#[test]
    fn test_utility_operations() {
        let primitives = make_primitives();
        
        // Test count
        let count_fn = primitives.get("count").unwrap();
        let res_count = count_fn(&[RVal::List(vec![RVal::Int(1), RVal::Int(2)])]).unwrap();
        assert_eq!(res_count, RVal::Int(2));

        // Test empty?
        let empty_fn = primitives.get("empty?").unwrap();
        assert_eq!(empty_fn(&[RVal::List(vec![])]).unwrap(), RVal::Bool(true));
        assert_eq!(empty_fn(&[RVal::List(vec![RVal::Int(1)])]).unwrap(), RVal::Bool(false));

        // Test peek
        let peek_fn = primitives.get("peek").unwrap();
        assert_eq!(peek_fn(&[RVal::List(vec![RVal::Int(1), RVal::Int(2)])]).unwrap(), RVal::Int(2));

        // Test range
        let range_fn = primitives.get("range").unwrap();
        let res_range = range_fn(&[RVal::Int(0), RVal::Int(3)]).unwrap();
        assert_eq!(res_range, RVal::List(vec![RVal::Int(0), RVal::Int(1), RVal::Int(2)]));
    }

#[test]
    fn test_type_check_operations() {
        let primitives = make_primitives();
        
        let is_vec = primitives.get("vector?").unwrap();
        let is_map = primitives.get("map?").unwrap();
        let is_num = primitives.get("number?").unwrap();

        assert_eq!(is_vec(&[RVal::List(vec![])]).unwrap(), RVal::Bool(true));
        assert_eq!(is_map(&[RVal::Map(HashMap::new())]).unwrap(), RVal::Bool(true));
        assert_eq!(is_num(&[RVal::Int(10)]).unwrap(), RVal::Bool(true));
        assert_eq!(is_num(&[RVal::Float(1.5)]).unwrap(), RVal::Bool(true));
        assert_eq!(is_num(&[RVal::Bool(true)]).unwrap(), RVal::Bool(false));
    }

}