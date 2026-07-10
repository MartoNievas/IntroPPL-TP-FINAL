/*

Module that implements the interpreter itself; it is responsible for executing the
code and communicating with the message interface used for inference.

*/

use super::machine::{Machine, Instr, Msg, Env, Addr, Closure};
use crate::parser::sexpr::Form;
use crate::parser::primitives::{is_primitive, make_primitives};
use crate::parser::value::RVal;
use crate::stats::as_f64;

// Executes instructions from the control stack until it encounters a probabilistic effect
pub fn resume(mut m: Machine) -> Result<Msg, String> {
    while let Some(instr) = m.c.pop() {
        match instr {
            Instr::Eval(expr, env, addr) => {
                match expr {
                    // 1. Atoms go straight to the value stack (V)
                    Form::Int(i) => m.v.push(RVal::Int(i)),
                    Form::Float(f) => m.v.push(RVal::Float(f)),
                    Form::Bool(b) => m.v.push(RVal::Bool(b)),
                    Form::Str(s) => m.v.push(RVal::Str(s)),
                    Form::Nil => m.v.push(RVal::Nil),

                    // 2. Symbols: look them up in the local/global environment or the primitives table
                    Form::Symbol(s) => {
                        if let Some(val) = env.get(&s) {
                            m.v.push(val.clone());
                        } else if let Some(val) = m.env.get(&s) { 
                            m.v.push(val.clone());
                        } else if is_primitive(&s) {
                            m.v.push(RVal::Str(s));
                        } else {
                            return Err(format!("Undefined variable or primitive function: '{}'", s));
                        }
                    }

                    // 3. Lists: special forms or function invocation

                    Form::List(list, _list_type) => {
                        if list.is_empty() {
                            m.v.push(RVal::List(vec![]));
                            continue;
                        }

                        if let Form::Symbol(head) = &list[0] {
                            match head.as_str() {
                                "let" => {
                                    // (let [var1 expr1 expr2 ...] body)
                                    if list.len() < 3 {
                                        return Err("Invalid 'let' syntax: expected (let [binds...] body...)".into());
                                    }
                                    if let Form::List(binds, _list_type) = &list[1] {
                                        let body = list[2..].to_vec();
                                        if binds.is_empty() {
                                            push_body(&mut m.c, &body, &env, addr);
                                        } else if binds.len() % 2 != 0 {
                                            return Err("Invalid 'let' bindings: the vector must contain an even number of elements (key-value pairs)".into());
                                        } else {
                                            let mut first_addr = addr.clone();
                                            first_addr.push("let_0".into());

                                            // Push the let's continuation and evaluate the first expression
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
                                        return Err("Invalid 'let' syntax: the first argument must be a list of bindings [...]".into());
                                    }
                                }

                                "if" => {
                                    if list.len() != 4 {
                                        return Err("Invalid 'if' syntax: expected (if test then else)".into());
                                    }
                                    let (test, then_b, else_b) = (&list[1], &list[2], &list[3]);
                                    let mut test_addr = addr.clone();
                                    test_addr.push("test".into());

                                    m.c.push(Instr::IfK(then_b.clone(), else_b.clone(), env.clone(), addr));
                                    m.c.push(Instr::Eval(test.clone(), env.clone(), test_addr));
                                }

                                "fn" => {
                                    if list.len() < 3 {
                                        return Err("Invalid 'fn' syntax: expected (fn [params...] body...)".into());
                                    }
                                    if let Form::List(params_form, _list_type) = &list[1] {
                                        let mut params = Vec::with_capacity(params_form.len());
                                        for p in params_form {
                                            if let Form::Symbol(param_name) = p {
                                                params.push(param_name.clone());
                                            } else {
                                                return Err("Invalid 'fn' syntax: all parameters must be symbols".into());
                                            }
                                        }
                                        let body = list[2..].to_vec();
                                        let closure = Closure { params, body, env: env.clone() };
                                        m.v.push(RVal::Closure(closure));
                                    } else {
                                        return Err("Invalid 'fn' syntax: expected a list of parameters as the first argument".into());
                                    }
                                }

                                "sample" => {
                                    if list.len() != 2 {
                                        return Err("Invalid 'sample' syntax: expected exactly 1 argument".into());
                                    }

                                    let mut d_addr = addr.clone();
                                    d_addr.push("d".into());
                                    m.c.push(Instr::SampleK(addr));
                                    m.c.push(Instr::Eval(list[1].clone(), env.clone(), d_addr));
                                }

                                "observe" => {
                                    if list.len() != 3 {
                                        return Err("Invalid 'observe' syntax: expected exactly 2 arguments".into());
                                    }
                                    let mut d_addr = addr.clone(); d_addr.push("d".into());
                                    let mut v_addr = addr.clone(); v_addr.push("v".into());
                                    m.c.push(Instr::ObserveK(addr));
                                    m.c.push(Instr::Eval(list[2].clone(), env.clone(), v_addr));
                                    m.c.push(Instr::Eval(list[1].clone(), env.clone(), d_addr));
                                }
                                
                                "factor" => {
                                    if list.len() != 2 {
                                        return Err("Invalid 'factor' syntax: expected exactly 1 argument".into());
                                    }

                                    let mut v_addr = addr.clone();
                                    v_addr.push("v".into());

                                    m.c.push(Instr::FactorK(addr));
                                    m.c.push(Instr::Eval(list[1].clone(), env.clone(), v_addr));
                                }

                                _ => {
                                    // Standard call to a function or primitive: (f arg1 arg2 ...)
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
                let val = m.v.pop().ok_or("Empty value stack (V) while evaluating LetK continuation")?;
                if let Form::Symbol(var_name) = &binds[2 * idx] {
                    env.insert(var_name.clone(), val);

                    if 2 * (idx + 1) < binds.len() {
                        // There are still variables left in the let
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
                        // All variables have been bound -> evaluate the body
                        push_body(&mut m.c, &body, &env, addr);
                    }
                } else {
                    return Err("Invalid 'let' binding: left-hand side assignment target must be a symbol".into());
                }
            }

            Instr::IfK(then_b, else_b, env, addr) => {
                let cond = m.v.pop().ok_or("Empty value stack (V) while evaluating IfK continuation")?;
                let branch = if matches!(cond, RVal::Bool(false) | RVal::Nil) { else_b } else { then_b };
                m.c.push(Instr::Eval(branch, env, addr));
            }

            Instr::CallK(n_args, addr) => {
                let mut args = Vec::with_capacity(n_args);
                for _ in 0..n_args {
                    args.push(m.v.pop().ok_or("Missing arguments on the value stack while evaluating CallK continuation")?);
                }
                args.reverse(); // Reverse the order so they are in the correct order

                let func = m.v.pop().ok_or("Missing function on the value stack while evaluating CallK continuation")?;

                // 1. Deterministic primitives by name
                if let RVal::Str(prim_name) = &func {
                    let prims = make_primitives();
                    if let Some(prim_fn) = prims.get(prim_name.as_str()) {
                        let res = prim_fn(&args)?;
                        m.v.push(res);
                    } else {
                        return Err(format!("Unknown primitive function: '{}'", prim_name));
                    }
                }
                // 2. Closure
                else if let RVal::Closure(f) = func {
                    if f.params.len() != args.len() {
                        return Err(format!("Arity mismatch: closure expected {} arguments, but received {}", f.params.len(), args.len()));
                    }
                    let mut new_env = f.env.clone();
                    for (param, arg) in f.params.iter().zip(args.into_iter()) {
                        new_env.insert(param.clone(), arg);
                    }
                    push_body(&mut m.c, &f.body, &new_env, addr);
                } else {
                    return Err(format!("Type error: attempted to invoke a non-callable value: {:?}", func));
                }
            }

            Instr::SampleK(addr) => {
                let dist_val = m.v.pop().ok_or("Missing distribution on the value stack while evaluating SampleK continuation")?;
                if let RVal::Dist(dist) = dist_val {
                    return Ok(Msg::Sample(addr, dist, m));
                } else {
                    return Err("Type error: 'sample' argument must evaluate to a Distribution object".into());
                }
            }

            Instr::ObserveK(addr) => {
                let y = m.v.pop().ok_or("Missing observed value on the value stack while evaluating ObserveK continuation")?;
                let dist_val = m.v.pop().ok_or("Missing distribution on the value stack while evaluating ObserveK continuation")?;

                if let RVal::Dist(dist) = dist_val {
                    return Ok(Msg::Observe(addr, dist, y, m));
                } else {
                    return Err("Type error: first argument to 'observe' must evaluate to a Distribution object".into());
                }
            }

            Instr::FactorK(addr) => {
                let val = m.v.pop().ok_or("Missing value on the value stack while evaluating FactorK continuation")?;
                let w = as_f64(&val)?;

                m.log_w += w;
                m.v.push(RVal::Nil);
            }

            Instr::Discard => {
                m.v.pop();
            }
        }
    }
    // If the C stack empties out completely, the program has finished
    let final_val = m.v.pop().unwrap_or(RVal::Nil);
    Ok(Msg::Done(final_val, m))
}

/// Injects a value into the stack of a paused machine (equivalent to `send` in Python).
pub fn send(m: &mut Machine, val: RVal) {
    m.v.push(val);
}

// Helper function to push a sequence of expressions
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