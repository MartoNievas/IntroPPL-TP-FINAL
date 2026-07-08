/*

Se define la libreria para utilizar las estructuras es todo el proyecto.

*/
pub mod parser; 
pub mod interpreter;
pub mod inference;


pub use interpreter::*;
pub use inference::*;
pub use parser::*;


