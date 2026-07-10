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

use std::env;

fn main() {
    println!("Starting Demonstration: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Author: Martín Nievas Wilberger");

    let args: Vec<String> = env::args().collect();
    let config = ppl_tp_final::cli::Config::parse_args(args);
    ppl_tp_final::runner::run(config);
}
