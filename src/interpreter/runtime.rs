use super::machine::{Machine, Instr, Msg, Env, Addr, Closure};
use crate::parser::sexpr::Form;
use crate::parser::primitives::{is_primitive, make_primitives};
use crate::parser::value::RVal;

// Funcion auxiliar para empujar secuancias de expresiones

fn push_body(c: &mut Vec<Instr>, body: &[Form], env: &Env, addr: Addr) {
    let n = body.len();
    if n == 0 {
        return ;
    }

    for (i, expr) in body.iter().enumerate().rev() {
        let mut sub_addr = addr.clone();
        sub_addr.push(format!("body_{}", i));

        if i < n - 1 {
            c.push(Instr::Discard);
        }
        c.push(Instr::Eval(expr.clone(), env.clone(), sub_addr));
    }
}

// Ejecuta instrucciones del stack de control hasta encontrar un efecto probabilistico
pub fn resume(mut m: Machine) -> Result<Msg, String> {
    while let Some(instr) = m.c.pop() {
        match instr {
            Instr::Eval(expr, env, addr) => {
                match expr {
                    // 1. Atomos van directamente al Stack de valores (V)
                    Form::Int(i) => m.v.push(RVal::Int(i)),
                    Form::Float(f) => m.v.push(RVal::Float(f)),
                    Form::Bool(b) => m.v.push(RVal::Bool(b)),
                    Form::Str(s) => m.v.push(RVal::Str(s)),
                    Form::Nil => m.v.push(RVal::Nil),

                    // 2. Simbolos: hacemos la busqueda en el entorno local/global o tabla de primitivas
                    Form::Symbol(s) => {
                        if let Some(val) = env.get(&s) {
                            m.v.push(val.clone());
                        } else if let Some(val) = m.env.get(&s) { 
                            m.v.push(val.clone());
                        } else if is_primitive(&s) {
                            m.v.push(RVal::Str(s));
                        } else {
                            return Err(format!("Not define primitive or variable: '{}'", s));
                        }
                    }

                    // 3. Listas: formas especiales o invocacion de funciones

                    Form::List(list) => {
                        if list.is_empty() {
                            m.v.push(RVal::List(vec![]));
                            continue;
                        }

                        if let Form::Symbol(head) = &list[0] {
                            match head.as_str() {
                                "let" => {
                                    // (let [var1 expr1 expr2 ...] body)
                                    if list.len() < 3 {
                                        return Err("Unexpected let syntaxis: (let [binds...] body...)".into());
                                    }
                                    if let Form::List(binds) = &list[1] {
                                        let body = list[2..].to_vec();
                                        if binds.is_empty() {
                                            push_body(&mut m.c, &body, &env, addr);
                                        } else if binds.len() % 2 != 0 {
                                            return Err("El vector de bindings en let debe tener un número par de elementos".into());
                                        } else {
                                            let mut first_addr = addr.clone();
                                            first_addr.push("let_0".into());

                                            // Empujamos la continuación del let y evaluamos la primera expresión
                                            m.c.push(Instr::LetK {
                                                binds: binds.clone(),
                                                idx: 0,
                                                body,
                                                env: env.clone(),
                                                addr: addr.clone(),
                                            });
                                            m.c.push(Instr::Eval(binds[1].clone(), env.clone(), first_addr));
                                        }
                                    } else {
                                        return Err("El primer argumento de let debe ser una lista de enlaces [...]".into());
                                    }
                                }

                                "if" => {
                                    if list.len() != 4 {
                                        return Err("Sintaxis de if invalida: (if test then else)".into());
                                    }
                                    let (test, then_b, else_b) = (&list[1], &list[2], &list[3]);
                                    let mut test_addr = addr.clone();
                                    test_addr.push("test".into());

                                    m.c.push(Instr::IfK(then_b.clone(), else_b.clone(), env.clone(), addr));
                                    m.c.push(Instr::Eval(test.clone(), env.clone(), test_addr));
                                }

                                "fn" => {
                                    if list.len() < 3 {
                                        return Err("Sintaxis de fn inválida: (fn [params...] body...)".into());
                                    }
                                    if let Form::List(params_form) = &list[1] {
                                        let mut params = Vec::with_capacity(params_form.len());
                                        for p in params_form {
                                            if let Form::Symbol(param_name) = p {
                                                params.push(param_name.clone());
                                            } else {
                                                return Err("Los parámetros de fn deben ser símbolos".into());
                                            }
                                        }
                                        let body = list[2..].to_vec();
                                        let closure = Closure { params, body, env: env.clone() };
                                        m.v.push(RVal::Closure(closure));
                                    } else {
                                        return Err("Se esperaba una lista de parámetros en fn".into());
                                    }
                                }

                                "sample" => {
                                    if list.len() != 2 {
                                        return Err("sample requiere exactamente un solo argumento".into());
                                    }

                                    let mut d_addr = addr.clone();
                                    d_addr.push("d".into());
                                    m.c.push(Instr::SampleK(addr));
                                    m.c.push(Instr::Eval(list[1].clone(), env.clone(), d_addr));
                                }

                                "observe" => {
                                    if list.len() != 3 {
                                        return Err("observe requiere exactamente 2 parametros".into());
                                    }
                                    let mut d_addr = addr.clone(); d_addr.push("d".into());
                                    let mut v_addr = addr.clone(); v_addr.push("v".into());
                                    m.c.push(Instr::ObserveK(addr));
                                    m.c.push(Instr::Eval(list[2].clone(), env.clone(), v_addr));
                                    m.c.push(Instr::Eval(list[1].clone(), env.clone(), d_addr));
                                }

                                _ => {
                                    // Llamada estandar a funcion o primitiva: (f arg1 arg2 ...)
                                    let n_args = list.len() - 1;
                                    m.c.push(Instr::CallK(n_args, addr.clone()));
                                    for i in (1..=n_args).rev() {
                                        let mut arg_addr = addr.clone();
                                        arg_addr.push(format!("arg_{}", i));
                                        m.c.push(Instr::Eval(list[i].clone(), env.clone(), arg_addr));
                                    }

                                    let mut fn_addr = addr.clone();
                                    fn_addr.push("fn".into());
                                    m.c.push(Instr::Eval(list[0].clone(), env.clone(), fn_addr));
                                }
                            }
                        } else {
                            let n_args = list.len() - 1;
                            m.c.push(Instr::CallK(n_args, addr.clone()));
                            for i in (1..=n_args).rev() {
                                let mut arg_addr = addr.clone();
                                arg_addr.push(format!("arg_{}", i));
                                m.c.push(Instr::Eval(list[i].clone(), env.clone(), arg_addr));
                            }

                            let mut fn_addr = addr.clone();
                            fn_addr.push("fn".into());
                            m.c.push(Instr::Eval(list[0].clone(), env.clone(), fn_addr));
                        }
                    }
                }
            }

            Instr::LetK { binds, idx, body, mut env, addr } => {
                let val = m.v.pop().ok_or("Stack V vacio al evaluar LetK")?;
                if let Form::Symbol(var_name) = &binds[2 * idx] {
                    env.insert(var_name.clone(), val);

                    if 2 * (idx + 1) < binds.len() {
                        // Aun quedan variables en el let
                        let next_idx = idx + 1;
                        let mut next_addr = addr.clone();
                        next_addr.push(format!("let_{}", 2 * next_idx));

                        m.c.push(Instr::LetK {
                            binds: binds.clone(),
                            idx: next_idx,
                            body,
                            env: env.clone(),
                            addr: addr.clone(),
                        });

                        m.c.push(Instr::Eval(binds[2 * next_idx + 1].clone(), env, next_addr));
                    } else {
                        // Todas las variables fueron enlazadas -> evaluar cuerpo
                        push_body(&mut m.c, &body, &env, addr);
                    }
                } else {
                    return Err("El lado izquierdo de una asignación en let debe ser un símbolo".into());
                }
            }

            Instr::IfK(then_b, else_b, env, addr) => {
                let cond = m.v.pop().ok_or("Stack V vacio en IfK")?;
                let branch = if matches!(cond, RVal::Bool(false) | RVal::Nil) { else_b } else { then_b };
                m.c.push(Instr::Eval(branch, env, addr));
            }

            Instr::CallK(n_args, addr) => {
                let mut args = Vec::with_capacity(n_args);
                for _ in 0..n_args {
                    args.push(m.v.pop().ok_or("Faltan argumentos en CallK")?);
                }
                args.reverse(); // Invertimos el orden para tenerlos en orden correcto

                let func = m.v.pop().ok_or("Falta la funcion en CallK")?;

                // 1. Primitivas deterministicas por nombre
                if let RVal::Str(prim_name) = &func {
                    let prims = make_primitives();
                    if let Some(prim_fn) = prims.get(prim_name.as_str()) {
                        let res = prim_fn(&args)?;
                        m.v.push(res);
                    } else {
                        return Err(format!("Primitiva desconocida: '{}'", prim_name));
                    }
                }
                // 2. Clousure
                else if let RVal::Closure(f) = func {
                    if f.params.len() != args.len() {
                        return Err(format!("Aridad incorrecta la funcion esperaba: {} argumentos pero recibio {} ", f.params.len(), args.len()));
                    }
                    let mut new_env = f.env.clone();
                    for (param, arg) in f.params.iter().zip(args.into_iter()) {
                        new_env.insert(param.clone(), arg);
                    }
                    push_body(&mut m.c, &f.body, &new_env, addr);
                } else {
                    return Err(format!("Se intentó invocar un valor que no es una función: {:?}", func));
                }
            }

            Instr::SampleK(addr) => {
                let dist_val = m.v.pop().ok_or("Falta distribución en SampleK")?;
                if let RVal::Dist(dist) = dist_val {
                    return Ok(Msg::Sample(addr, dist, m));
                } else {
                    return Err("El argumento de sample debe evaluar a una distribución".into());
                }
            }

            Instr::ObserveK(addr) => {
                let y = m.v.pop().ok_or("Falta el valor observado en ObserveK")?;
                let dist_val = m.v.pop().ok_or("Falta distribución en ObserveK")?;

                if let RVal::Dist(dist) = dist_val {
                    let y_f64 = match y {
                        RVal::Float(f) => f,
                        RVal::Int(i) => i as f64,
                        _ => return Err("El valor observado en observe debe ser un numerico".into()),
                    };

                    return Ok(Msg::Observe(addr, dist, y_f64, m));
                } else {
                    return Err("El primer argumento de observe debe ser una distribución".into());
                }
            }

            Instr::Discard => {
                m.v.pop();
            }
        }
    }
    // Si el stack C se vacía por completo, el programa terminó
    let final_val = m.v.pop().unwrap_or(RVal::Nil);
    Ok(Msg::Done(final_val, m))
}

/// Inyecta un valor en el stack de una máquina pausada (equivalente a send en Python).
pub fn send(m: &mut Machine, val: RVal) {
    m.v.push(val);
}
