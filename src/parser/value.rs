/* Objeto que sirve como valor de retorno de todas la primitivas incluidas distribuciones*/

use std::{collections::HashMap};
 
use ndarray::{Array2};
 
use crate::parser::distribution::{Distribution};

use crate::parser::primitives::HashKey;

use crate::interpreter::machine::Closure as MClosure;

#[derive(Debug, Clone)]
pub enum RVal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
    List(Vec<RVal>),
    Map(HashMap<HashKey, RVal>),
    Matrix(Array2<f64>),
    Dist(Distribution),

    Closure(MClosure),
}
 
impl PartialEq for RVal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RVal::Int(a), RVal::Int(b)) => a == b,
            (RVal::Float(a), RVal::Float(b)) => a == b,
            (RVal::Int(a), RVal::Float(b)) | (RVal::Float(b), RVal::Int(a)) => (*a as f64) == *b,
            (RVal::Bool(a), RVal::Bool(b)) => a == b,
            (RVal::Str(a), RVal::Str(b)) => a == b,
            (RVal::Nil, RVal::Nil) => true,
            (RVal::List(a), RVal::List(b)) => a == b,
            (RVal::Matrix(a), RVal::Matrix(b)) => a == b,
            _ => false,
        }
    }
}

 
impl std::fmt::Display for RVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RVal::Int(i) => write!(f, "{i}"),
            RVal::Float(v) => write!(f, "{v}"),
            RVal::Bool(b) => write!(f, "{b}"),
            RVal::Str(s) => write!(f, "{s}"),
            RVal::Nil => write!(f, "nil"),
            RVal::List(v) => {
                write!(f, "[")?;
                for (i, x) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{x}")?;
                }
                write!(f, "]")
            }
            RVal::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            RVal::Matrix(m) => write!(f, "Matrix{m:?}"),
            RVal::Dist(d) => write!(f, "{d}"),
            RVal::Closure(c) => write!(f, "<fn [{}]>", c.params.join(", ")),
        }
    }
}