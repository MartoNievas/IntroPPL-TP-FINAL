/*Libreria para el uso de las estructuras*/
pub mod parser; 
pub use parser::distribution::{Distribution};
pub use parser::primitives::{make_primitives, is_primitive, HashKey};
pub use parser::value::RVal;
pub use parser::sexpr::*;