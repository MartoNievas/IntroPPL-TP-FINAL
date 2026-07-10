/*

Se define la libreria para utilizar las estructuras es todo el proyecto.

*/
pub mod parser; 
pub mod interpreter;
pub mod inference;
pub mod stats;
pub mod ui;
pub mod cli;
pub mod demos;
pub mod runner;

pub use interpreter::*;
pub use inference::*;
pub use parser::*;
