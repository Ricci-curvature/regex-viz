//! Stage 3 CLI: regex → subset-construction trace JSON on stdout.
//!
//!   cargo run --example 03_subset_construction -- "(a|b)*c"
//!
//! Emits a `ConstructionTrace`: source NFA + alphabet Σ + an accumulated
//! sequence of `ConstructionStep`s covering every subset discovery and every
//! DFA transition added. Consumed by `tools/build_artifacts.sh` and by the
//! React viewer.
//!
//! Exit codes: 0 success, 1 error, 2 usage error.

use std::env;
use std::process::ExitCode;

use regex_viz::dfa;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "usage: {} <regex>",
            args.first()
                .map(String::as_str)
                .unwrap_or("03_subset_construction")
        );
        return ExitCode::from(2);
    }
    let trace = match dfa::construct(&args[1]) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&trace) {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("serialize error: {e}");
            ExitCode::from(1)
        }
    }
}
