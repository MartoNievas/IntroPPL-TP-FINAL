/*Libreria para el uso de las estructuras*/
pub mod parser; 
pub mod interpreter;
pub use interpreter::machine::*;
pub use interpreter::runtime::*;
pub use parser::distribution::{Distribution};
pub use parser::primitives::*;
pub use parser::value::RVal;
pub use parser::sexpr::*;
