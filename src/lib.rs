/*Libreria para el uso de las estructuras*/
pub mod parser; 
pub use parser::distribution::{Distribution, Value};
pub use parser::primitives::{make_primitives, is_primitive, RVal, HashKey};