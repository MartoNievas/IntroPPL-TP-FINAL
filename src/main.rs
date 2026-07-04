mod parser;
mod interpreter;
mod inference;
use std::env::args;

use PPL_TP_FINAL::lw::likelihood_weighting;

use crate::inference::lw::run_lw;
use crate::parser::*;
use crate::inference::*;
use crate::interpreter::*;
use std::env;


fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        return Err(format!("Usage: {} <source_code>", args[0]));
    }

    let program = args[1].clone();

    let initial_machine = initial_machine(&program);


    print!("Success");
    Ok(())
}
