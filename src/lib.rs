/*Libreria para el uso de las estructuras*/
pub mod parser; 
pub mod interpreter;
pub mod inference;

pub use inference::*;
pub use interpreter::*;
pub use parser::distribution::{Distribution};
pub use parser::primitives::*;
pub use parser::value::RVal;
pub use parser::sexpr::*;
