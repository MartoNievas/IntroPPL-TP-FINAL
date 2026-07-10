/*

Module dedicated to parsing the arguments provided via standard input to decide the
execution mode of the program.
It also parses the inference algorithm to use in probabilistic file mode.

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

// Already validated/derived configuration from the command-line arguments.
// `main` only builds this and passes it to `runner::run`; all the logic for
// interpreting argv lives here, not in main.
#[derive(Debug, Clone)]
pub enum Config {
    /// None -> runs all demos; Some(n) -> runs demo n.
    Demo(Option<usize>),
    /// (file, inference algorithm)
    File(String, Algorithm),
    /// file without algorithm, assumed to be deterministic
    Deterministic(String),

    /// invalid argv; the String is the error message to show before print_usage
    Invalid(String),
}

impl Config {
    pub fn parse_args(args: Vec<String>) -> Self {
        // File + algorithm mode: cargo run -- <file.hoppl> <algorithm>
        if args.len() >= 3 && args[1].parse::<usize>().is_err() {
            let file_path = args[1].clone();
            let algo_name = &args[2];

            return match Algorithm::parse(algo_name) {
                Some(algorithm) => Config::File(file_path, algorithm),
                None => Config::Invalid(format!("Unknown algorithm: '{algo_name}'")),
            };
        }

        // Deterministic file mode: cargo run -- <file.hoppl>  (no algorithm)
        if args.len() == 2 && args[1].parse::<usize>().is_err() {
            return Config::Deterministic(args[1].clone());
        }

        // Demo mode: cargo run [-- <number>]
        match args.get(1) {
            None => Config::Demo(None),
            Some(raw) => match raw.parse::<usize>() {
                Ok(n) => {
                    let demos = build_demos();
                    if demos.iter().any(|d| d.id == n) {
                        Config::Demo(Some(n))
                    } else {
                        Config::Invalid(format!(
                            "Invalid demo number: {n}. Use a value between 1 and {}.",
                            demos.len()
                        ))
                    }
                }
                Err(_) => Config::Invalid(format!("Unrecognized argument: '{raw}'")),
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
    eprintln!("Usage:");
    eprintln!("  cargo run                                -> runs all hardcoded demos");
    eprintln!(
        "  cargo run -- <number>                    -> runs a specific demo (1-{})",
        demos.len()
    );
    eprintln!(
        "  cargo run -- <file.hoppl>                -> runs a deterministic model (no sample/observe)"
    );
    eprintln!(
        "  cargo run -- <file.hoppl> <algorithm>    -> runs a probabilistic model with the given algorithm"
    );
    eprintln!();
    eprintln!(
        "Available algorithms: lw, ssmh, smc, bbvi, exact-enumeration (alias: enum, exact)"
    );
    eprintln!();
    eprintln!("Available demos:");
    for d in demos {
        eprintln!("   {}: {}", d.id, d.label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        // args[0] is the binary name, as in env::args()
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
        match Config::parse_args(args(&["model.hoppl"])) {
            Config::Deterministic(path) => assert_eq!(path, "model.hoppl"),
            other => panic!("expected Deterministic, got {other:?}"),
        }
    }

    #[test]
    fn config_file_with_algorithm() {
        match Config::parse_args(args(&["model.hoppl", "lw"])) {
            Config::File(path, algo) => {
                assert_eq!(path, "model.hoppl");
                assert!(matches!(algo, Algorithm::Lw));
            }
            other => panic!("expected File, got {other:?}"),
        }
    }

    #[test]
    fn config_file_with_unknown_algorithm_is_invalid() {
        assert!(matches!(
            Config::parse_args(args(&["model.hoppl", "gibbs"])),
            Config::Invalid(_)
        ));
    }
}