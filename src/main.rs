/*

Program entry point module, for running a demonstration or running HOPPL
code files.
With the following usage modes:

    - Demo mode: cargo run -> runs the full demo | cargo run -- <demo_number> -> runs a specific demo.
    - Deterministic execution: cargo run -- <file.hoppl> -> runs a program without 'sample'/'observe'.
    - Probabilistic HOPPL code execution: cargo run -- <file.hoppl> <inference algorithm>.

    number of demos = 6
    inference algorithms:
        - lw
        - ssmh
        - smc
        - bbvi
        - exact enumeration

*/

mod cli;
mod demos;
mod inference;
mod interpreter;
mod parser;
mod runner;
mod stats;
mod ui;

use std::env;

use cli::Config;

fn main() {
    println!("Starting Demonstration: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Author: Martín Nievas Wilberger");

    let args: Vec<String> = env::args().collect();
    let config = Config::parse_args(args);
    runner::run(config);
}