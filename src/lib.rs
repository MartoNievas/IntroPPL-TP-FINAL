/*

Se define la libreria para utilizar las estructuras es todo el proyecto.

*/
pub mod parser; 
pub mod interpreter;
pub mod inference;

pub use inference::*;
pub use interpreter::*;
pub use parser::distribution::{Distribution};
pub use parser::primitives::*;
pub use parser::value::RVal;
pub use parser::sexpr::*;
