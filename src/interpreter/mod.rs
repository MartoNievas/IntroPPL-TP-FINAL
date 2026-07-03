pub mod machine;
pub mod runtime;


pub use machine::{Machine, Instr, Msg, Env, Addr, Closure};
pub use runtime::{resume, send};

use crate::parser::sexpr::{parse, Form};
use crate::parser::value::RVal;

/// Configura la máquina inicial, extrayendo funciones globales (defn) al entorno.
pub fn initial_machine(program: &str) -> Result<Machine, String> {
    let forms = parse(program)?;
    let mut genv = Env::new();
    let mut main_form = None;

    for form in forms {
        if let Form::List(list) = &form {
            if !list.is_empty() {
                if let Form::Symbol(sym) = &list[0] {
                    if sym == "defn" {
                        // Sintaxis: (defn nombre [params...] body...)
                        if list.len() < 4 {
                            return Err("Sintaxis inválida para defn".into());
                        }
                        if let Form::Symbol(name) = &list[1] {
                            if let Form::List(params_form) = &list[2] {
                                let mut params = Vec::new();
                                for p in params_form {
                                    if let Form::Symbol(param_name) = p {
                                        params.push(param_name.clone());
                                    } else {
                                        return Err("Los parámetros en defn deben ser símbolos".into());
                                    }
                                }
                                let body = list[3..].to_vec();
                                let closure = Closure { params, body, env: genv.clone() };
                                genv.insert(name.clone(), RVal::Closure(closure));
                                continue;
                            }
                        }
                    }
                }
            }
        }
        // Si no es un defn, asumimos que es la expresión principal a evaluar
        main_form = Some(form);
    }

    let main_expr = main_form.ok_or("No se encontró una expresión principal para evaluar en el programa")?;
    let initial_instr = Instr::Eval(main_expr, genv.clone(), Vec::new());
    
    Ok(Machine::new(vec![initial_instr], genv))
}