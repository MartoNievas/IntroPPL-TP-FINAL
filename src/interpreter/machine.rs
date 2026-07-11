/*

Module that implements the state machine used to execute a HOPPL program. It is
responsible for tracking the execution stacks and the global Environment where the
primitives live, as well as the local one. It also implements the functionality to
clone a machine's state, which SMC relies on.

*/

use std::collections::HashMap;
use crate::parser::sexpr::Form;
use crate::parser::value::RVal;
use crate::parser::distribution::Distribution;

pub type Env = HashMap<String, RVal>;
pub type Addr = Vec<String>;

#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub params: Vec<String>,
    pub body: Vec<Form>,
    pub env: Env,
}

// Here we introduce the control stack.
// Each *K variant represents a continuation frame that is still pending execution.

#[derive(Debug, Clone)]
pub enum Instr {

    // Evaluate an expression in the given environment and address
    Eval(Form, Env, Addr),

    // Continuation for `let`: process the next binding or evaluate the body
    LetK {
        binds: Vec<Form>,
        idx: usize,
        body: Vec<Form>,
        env: Env,
        addr: Addr,
    },

    // Continuation for `if`: evaluate the `then` or `else` branch depending
    // on the value of the predicate
    IfK(Form, Form, Env, Addr),

    // Continuation for a primitive function call: `callk` with the number of arguments as usize
    CallK(usize, Addr),

    // Continuation for probabilistic sampling
    SampleK(Addr),

    // Continuation for probabilistic observation
    ObserveK(Addr),

    // Continuation for soft conditioning via the `factor` operator:
    // adds a term directly to the accumulated log-weight instead of
    // pausing the machine for the inference engine to intervene.
    FactorK(Addr),

    // Discard the last value evaluated from the stack
    Discard,
}

// Message returned by `resume()` when the machine pauses due to an effect or finishes
#[derive(Debug)]
pub enum Msg {
    Sample(Addr, Distribution, Machine),
    Observe(Addr, Distribution, RVal, Machine),
    Factor(Addr, f64, Machine),
    Done(RVal, Machine),   
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub c: Vec<Instr>,
    pub v: Vec<RVal>,
    pub env: Env,
    pub log_w: f64,
}

impl Machine {
    pub fn new(c: Vec<Instr>, env: Env) -> Self {
        Machine {
            c,
            v: Vec::new(),
            env,
            log_w: 0.0,
        }
    }

    // Clones the machine's exact state at a given instant.
    // This is essential for SMC and SSMH
    pub fn fork(&self) -> Self {
        self.clone()
    }
}