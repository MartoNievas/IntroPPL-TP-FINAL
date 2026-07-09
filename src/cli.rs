/*



*/
use crate::demos::{build_demos, Demo};

#[derive(Debug, Clone, Copy)]
pub enum Algorithm {
    Lw,
    Ssmh,
    Smc,
    Bbvi,
    Enumeration,
}

// Configuracion ya validada/derivada de los argumentos de linea de comando.
// `main` solo construye esto y lo pasa a `runner::run`; toda la logica de
// interpretacion de argv vive aca, no en main.
#[derive(Debug, Clone)]
pub enum Config {
    /// None -> corre todas las demos; Some(n) -> corre la demo n.
    Demo(Option<usize>),
    /// (archivo, algoritmo de inferencia)
    File(String, Algorithm),
    /// archivo sin algoritmo, se asume deterministico
    Deterministic(String),
    /// argv invalido; el String es el mensaje de error a mostrar antes de print_usage
    Invalid(String),
}

impl Config {
    pub fn parse_args(args: Vec<String>) -> Self {
        // Modo archivo + algoritmo: cargo run -- <archivo.hoppl> <algoritmo>
        if args.len() >= 3 && args[1].parse::<usize>().is_err() {
            let file_path = args[1].clone();
            let algo_name = &args[2];

            return match Algorithm::parse(algo_name) {
                Some(algorithm) => Config::File(file_path, algorithm),
                None => Config::Invalid(format!("Algoritmo desconocido: '{algo_name}'")),
            };
        }

        // Modo archivo deterministico: cargo run -- <archivo.hoppl>  (sin algoritmo)
        if args.len() == 2 && args[1].parse::<usize>().is_err() {
            return Config::Deterministic(args[1].clone());
        }

        // Modo demo: cargo run [-- <numero>]
        match args.get(1) {
            None => Config::Demo(None),
            Some(raw) => match raw.parse::<usize>() {
                Ok(n) => {
                    let demos = build_demos();
                    if demos.iter().any(|d| d.id == n) {
                        Config::Demo(Some(n))
                    } else {
                        Config::Invalid(format!(
                            "Numero de demo invalido: {n}. Usa un valor entre 1 y {}.",
                            demos.len()
                        ))
                    }
                }
                Err(_) => Config::Invalid(format!("Argumento no reconocido: '{raw}'")),
            },
        }
    }
}

impl Algorithm {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().replace('_', "-").as_str() {
            "lw" => Some(Algorithm::Lw),
            "ssmh" => Some(Algorithm::Ssmh),
            "smc" => Some(Algorithm::Smc),
            "bbvi" => Some(Algorithm::Bbvi),
            "exact-enumeration" | "exact enumeration" | "enum" | "exact" | "enumeration" => {
                Some(Algorithm::Enumeration)
            }
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Algorithm::Lw => "Likelihood Weighting",
            Algorithm::Ssmh => "Single-Site Metropolis-Hastings",
            Algorithm::Smc => "Sequential Monte Carlo",
            Algorithm::Bbvi => "Black-Box Variational Inference",
            Algorithm::Enumeration => "Exact Enumeration",
        }
    }
}

pub fn print_usage(demos: &[Demo]) {
    eprintln!("Uso:");
    eprintln!("  cargo run                                -> corre todas las demos hardcodeadas");
    eprintln!(
        "  cargo run -- <numero>                    -> corre una demo especifica (1-{})",
        demos.len()
    );
    eprintln!(
        "  cargo run -- <archivo.hoppl>             -> corre un modelo deterministico (sin sample/observe)"
    );
    eprintln!(
        "  cargo run -- <archivo.hoppl> <algoritmo> -> corre un modelo probabilistico con el algoritmo dado"
    );
    eprintln!();
    eprintln!(
        "Algoritmos disponibles: lw, ssmh, smc, bbvi, exact-enumeration (alias: enum, exact)"
    );
    eprintln!();
    eprintln!("Demos disponibles:");
    for d in demos {
        eprintln!("   {}: {}", d.id, d.label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        // args[0] es el nombre del binario, como en env::args()
        std::iter::once("hoppl".to_string())
            .chain(v.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn parse_accepts_known_aliases() {
        assert!(matches!(Algorithm::parse("lw"), Some(Algorithm::Lw)));
        assert!(matches!(Algorithm::parse("SSMH"), Some(Algorithm::Ssmh)));
        assert!(matches!(
            Algorithm::parse("exact_enumeration"),
            Some(Algorithm::Enumeration)
        ));
        assert!(matches!(Algorithm::parse("enum"), Some(Algorithm::Enumeration)));
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(Algorithm::parse("gibbs").is_none());
    }

    #[test]
    fn config_no_args_runs_all_demos() {
        assert!(matches!(Config::parse_args(args(&[])), Config::Demo(None)));
    }

    #[test]
    fn config_valid_demo_number() {
        assert!(matches!(
            Config::parse_args(args(&["3"])),
            Config::Demo(Some(3))
        ));
    }

    #[test]
    fn config_out_of_range_demo_number_is_invalid() {
        assert!(matches!(
            Config::parse_args(args(&["99"])),
            Config::Invalid(_)
        ));
    }

    #[test]
    fn config_file_without_algorithm_is_deterministic() {
        match Config::parse_args(args(&["modelo.hoppl"])) {
            Config::Deterministic(path) => assert_eq!(path, "modelo.hoppl"),
            other => panic!("esperaba Deterministic, obtuve {other:?}"),
        }
    }

    #[test]
    fn config_file_with_algorithm() {
        match Config::parse_args(args(&["modelo.hoppl", "lw"])) {
            Config::File(path, algo) => {
                assert_eq!(path, "modelo.hoppl");
                assert!(matches!(algo, Algorithm::Lw));
            }
            other => panic!("esperaba File, obtuve {other:?}"),
        }
    }

    #[test]
    fn config_file_with_unknown_algorithm_is_invalid() {
        assert!(matches!(
            Config::parse_args(args(&["modelo.hoppl", "gibbs"])),
            Config::Invalid(_)
        ));
    }
}