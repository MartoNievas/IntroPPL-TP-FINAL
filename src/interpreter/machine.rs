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

// Aqui introducimos el stack de control
// Cada variante *K representa un frame de continuacion pediente por ejecutar

#[derive(Debug, Clone)]
pub enum Instr {

    // Evaluar una expresion en el entorno y direccion dados
    Eval(Form, Env, Addr),

    // Continuacion de `let`: procesar el siguiente enlace o evaluar el cuerpo
    LetK {
        binds: Vec<Form>,
        idx: usize,
        body: Vec<Form>,
        env: Env,
        addr: Addr,
    },

    // Continuacion de `if`: evaluar la rama `then` o `else` segun
    // el valor del predicado
    IfK(Form, Form, Env, Addr),

    // Continuacion a funcion primitiva: `callk` con el numero de argumentos de usize
    CallK(usize, Addr),

    // Continuacion de muestreo probabilistico
    SampleK(Addr),

    // Continuacion de observacion probabilistica
    ObserveK(Addr),

    // Descartar el ultimo valor evaluado del stack
    Discard,
}

// Mensaje devuelto por `resume()` cuando la maquina se pausa por un efecto o termina
#[derive(Debug)]
pub enum Msg {
    Sample(Addr, Distribution, Machine),
    Observe(Addr, Distribution, f64, Machine),
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

    // clona el estado exacto de la maquina en un instante dado
    // Es vital para SMC y SSMH
    pub fn fork(&self) -> Self {
        self.clone()
    }
}