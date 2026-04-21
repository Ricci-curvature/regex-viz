//! Stage 4 CLI: regex + input → comparison-trace JSON on stdout.
//!
//!   cargo run --example 04_compare_nfa_dfa -- "(a|b)*c" "aabc"
//!
//! Emits a `ComparisonTrace`: source NFA + final DFA + per-step active-set
//! snapshots for both engines + a `summary` asserting they reach the same
//! verdict. Consumed by `tools/build_artifacts.sh` and by `ComparisonViewer`.
//!
//! Exit codes: 0 success, 1 error, 2 usage error.

use std::env;
use std::process::ExitCode;

use regex_viz::comparison;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: {} <regex> <input>",
            args.first()
                .map(String::as_str)
                .unwrap_or("04_compare_nfa_dfa")
        );
        return ExitCode::from(2);
    }
    let trace = match comparison::run_comparison(&args[1], &args[2]) {
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
