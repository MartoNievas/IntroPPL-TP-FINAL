/*

Módulo de entrada del programa, para ejecución de una demostración o ejecución de archivos de codigo del hoppl.
Con los siguientes modos de uso:

    - Modo demostracion: cargo run -> corre la demo entera | cargo run -- <num_demo> -> corre demo especifica.
    - Ejecución determinística: cargo run -- <archivo.hoppl> -> corre un programa sin 'sample'/'observe'.
    - Ejecución de código hoppl probabilístico: cargo run -- <archivo.hoppl> <algoritmo de inferencia>.

    numero de demos = 6
    algoritmos de inferencia:
        - lw
        - ssmh
        - smc
        - bbvi
        - exact numeration

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
    println!("Iniciando Demostracion: HOPPL (Higher-Order Probabilistic Programming Language)");
    println!("Autor: Martin Nievas Wilberger");

    let args: Vec<String> = env::args().collect();
    let config = Config::parse_args(args);
    runner::run(config);
}