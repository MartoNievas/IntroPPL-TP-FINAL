/*

Modulo que implementa formateo de colores, headers, etc para la impresion por terminal.

*/

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";

pub fn pause() {
    print!("\n   Presiona ENTER para continuar...");
    use std::io::{self, Write};
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
}

pub fn print_header(title: &str) {
    println!("\n{BOLD}{CYAN}{:=^80}{RESET}", format!(" {title} "));
}

pub fn print_ok(msg: &str) {
    println!("   {GREEN}[OK]{RESET} {msg}");
}

pub fn print_err(msg: &str) {
    println!("   {RED}[ERROR]{RESET} {msg}");
}

pub fn print_warn(msg: &str) {
    println!("   {YELLOW}[AVISO]{RESET} {msg}");
}

pub fn fmt_log_mass(log_mass: f64) -> String {
    if log_mass == f64::NEG_INFINITY {
        "-∞".to_string()
    } else if log_mass.is_infinite() {
        "∞".to_string()
    } else {
        format!("{:.4}", log_mass)
    }
}