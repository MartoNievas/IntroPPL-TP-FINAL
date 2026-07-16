/*

Shared default parameters for each inference algorithm. Used both by the
plain CLI file mode (`runner.rs::run_algorithm_on_model`) and by the
terminal debugger, so that stepping through a model in debug mode uses
the exact same particle counts / step budgets as running it normally.

*/

pub const N_PARTICLES_LW: usize = 5000;
pub const N_PARTICLES_SMC: usize = 2000;
pub const SSMH_STEPS: usize = 4000;
pub const SSMH_WARMUP: usize = 1000;
pub const BBVI_STEPS: usize = 250;
pub const BBVI_SAMPLES: usize = 20;
pub const BBVI_LR: f64 = 0.05;
pub const ENUM_MAX_TRACES: usize = 100000;
