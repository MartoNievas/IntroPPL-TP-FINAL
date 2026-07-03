use PPL_TP_FINAL::interpreter::{initial_machine, resume, send, Msg, Machine};
use PPL_TP_FINAL::parser::value::RVal;

fn run_to_done(code: &str) -> Result<RVal, String> {
    let m = initial_machine(code)?;
    match resume(m)? {
        Msg::Done(val, _) => Ok(val),
        Msg::Sample(_, _, _) => Err("Se obtuvo Msg::Sample inesperadamente en un test determinístico".into()),
        Msg::Observe(_, _, _, _) => Err("Se obtuvo Msg::Observe inesperadamente en un test determinístico".into()),
    }
}

#[cfg(test)]
mod deterministic_evaluation_tests {
    use super::*;

    #[test]
    fn test_literals_and_basic_math() {
        assert_eq!(run_to_done("42").unwrap(), RVal::Int(42));
        assert_eq!(run_to_done("3.14").unwrap(), RVal::Float(3.14));
        assert_eq!(run_to_done("true").unwrap(), RVal::Bool(true));
        assert_eq!(run_to_done("nil").unwrap(), RVal::Nil);
        assert_eq!(run_to_done("\"hola\"").unwrap(), RVal::Str("hola".to_string()));

        assert_eq!(run_to_done("(+ 5 3)").unwrap(), RVal::Int(8));
        assert_eq!(run_to_done("(* (- 10 2) 3)").unwrap(), RVal::Int(24));
        assert_eq!(run_to_done("(/ 20 4)").unwrap(), RVal::Int(5));
    }

    #[test]
    fn test_let_binding_sequential() {
        let code1 = "(let [x 10] (+ x 5))";
        assert_eq!(run_to_done(code1).unwrap(), RVal::Int(15));

        let code2 = "(let [a 5 b (+ a 5)] (* a b))";
        assert_eq!(run_to_done(code2).unwrap(), RVal::Int(50));

        let code3 = "(let [] 99)";
        assert_eq!(run_to_done(code3).unwrap(), RVal::Int(99));
    }

    #[test]
    fn test_if_conditional_branching() {
        assert_eq!(run_to_done("(if true 100 200)").unwrap(), RVal::Int(100));
        assert_eq!(run_to_done("(if false 100 200)").unwrap(), RVal::Int(200));
        
        let code_lazy = "(if true 42 (/ 1 0))";
        assert_eq!(run_to_done(code_lazy).unwrap(), RVal::Int(42));

        assert_eq!(
            run_to_done("(if (< 5 10) \"menor\" \"mayor\")").unwrap(),
            RVal::Str("menor".to_string())
        );
    }
}

#[cfg(test)]
mod higher_order_functions_tests {
    use super::*;

    #[test]
    fn test_anonymous_functions_and_application() {
        let code = "((fn [x y] (+ x y)) 10 20)";
        assert_eq!(run_to_done(code).unwrap(), RVal::Int(30));
    }

    #[test]
    fn test_closures_and_lexical_scoping() {
        let code = "(let [make-shift (fn [mu] (fn [x] (+ x mu))) \
                          f (make-shift 10)] \
                      (f 3))";
        assert_eq!(run_to_done(code).unwrap(), RVal::Int(13));
    }

    #[test]
    fn test_defn_and_recursion() {
        let code = "(defn fact [n] \
                      (if (<= n 1) \
                          1 \
                          (* n (fact (- n 1))))) \
                    (fact 5)";
        assert_eq!(run_to_done(code).unwrap(), RVal::Int(120));
    }
}

#[cfg(test)]
mod probabilistic_effects_tests {
    use super::*;

    #[test]
    fn test_sample_pauses_and_resumes() {
        let code = "(let [x (sample (normal 0 1))] (+ x 10))";
        let m = initial_machine(code).unwrap();

        match resume(m).unwrap() {
            Msg::Sample(addr, dist, mut paused_m) => {
                assert!(!addr.is_empty());
                assert_eq!(dist.to_string(), "(normal 0 1)");

                send(&mut paused_m, RVal::Float(5.0));

                match resume(paused_m).unwrap() {
                    Msg::Done(val, _) => assert_eq!(val, RVal::Float(15.0)),
                    _ => panic!("Se esperaba Msg::Done tras reanudar el sample"),
                }
            }
            _ => panic!("Se esperaba Msg::Sample al evaluar el programa"),
        }
    }

    #[test]
    fn test_observe_pauses_and_resumes() {
        let code = "(let [mu 2.0] (observe (normal mu 1) 2.5) mu)";
        let m = initial_machine(code).unwrap();

        match resume(m).unwrap() {
            Msg::Observe(addr, dist, y_obs, mut paused_m) => {
                assert!(!addr.is_empty());
                assert_eq!(dist.to_string(), "(normal 2 1)");
                assert_eq!(y_obs, 2.5);

                send(&mut paused_m, RVal::Float(y_obs));

                match resume(paused_m).unwrap() {
                    Msg::Done(val, _) => assert_eq!(val, RVal::Float(2.0)),
                    _ => panic!("Se esperaba Msg::Done tras reanudar el observe"),
                }
            }
            _ => panic!("Se esperaba Msg::Observe al evaluar el programa"),
        }
    }

    #[test]
    fn test_multiple_effects_in_sequence() {
        let code = "(let [a (sample (normal 0 1)) \
                          b (sample (normal a 1))] \
                      (+ a b))";
        let m = initial_machine(code).unwrap();

        let mut m1 = match resume(m).unwrap() {
            Msg::Sample(_, _, paused_m) => paused_m,
            _ => panic!("Fallo en el primer sample"),
        };
        send(&mut m1, RVal::Float(3.0));

        let mut m2 = match resume(m1).unwrap() {
            Msg::Sample(_, _, paused_m) => paused_m,
            _ => panic!("Fallo en el segundo sample"),
        };
        send(&mut m2, RVal::Float(4.0));

        match resume(m2).unwrap() {
            Msg::Done(val, _) => assert_eq!(val, RVal::Float(7.0)),
            _ => panic!("Fallo en la evaluación final"),
        }
    }
}