/*

Object that serves as the return value of all primitives, including distributions.
This is implemented because we don't have Python's dynamic typing, so instead we
use Rust's algebraic types/enums.

*/

use std::hash::Hash;
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

impl RVal {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            RVal::Float(f) => Some(*f),
            RVal::Int(i) => Some(*i as f64),
            RVal::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            RVal::Float(f) => Some(*f as i64),
            RVal::Bool(b) => if *b { Some(1) } else { Some(0) },
            RVal::Int(i) => Some(*i),
            _ => None,
        }
    }
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
            RVal::Float(v) => write!(f, "{:?}",v),
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


// We implement the Eq and Hash traits for RVal to optimize the exact enumeration table

impl Eq for RVal {}

impl Hash for RVal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            RVal::Int(i) => i.hash(state),
            RVal::Float(f) => f.to_bits().hash(state), // Convert to bits to hash floats
            RVal::Bool(b) => b.hash(state),
            RVal::Str(s) => s.hash(state),
            RVal::Nil => ().hash(state),
            RVal::List(v) => v.hash(state),
            RVal::Map(m) => {
                // Maps are hard to hash because of ordering,
                // but if HashKey allows it, we use its size
                m.len().hash(state);
            }
            // For complex types like Matrix, Closure, or Dist,
            // it isn't advisable to use them as HashMap keys.
            // If that happens anyway, we hash their address or just skip them
            _ => 0.hash(state),
        }
    }
}