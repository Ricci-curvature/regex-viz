//! Stage 5 CLI: regex → minimization-trace JSON on stdout.
//!
//!   cargo run --example 05_minimize_dfa -- "(a|b)*c"
//!
//! Emits a `MinimizationTrace`: source NFA + source DFA (from Stage 3) +
//! per-step Hopcroft partition snapshots + the minimized DFA. Missing
//! transitions in the source DFA are treated as going to an implicit dead
//! sink; the minimized DFA exposes `sink_block` so viewers can hide it.
//! Consumed by `tools/build_artifacts.sh` and by `MinimizationViewer`.
//!
//! Exit codes: 0 success, 1 error, 2 usage error.

use std::env;
use std::process::ExitCode;

use regex_viz::minimize;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "usage: {} <regex>",
            args.first()
                .map(String::as_str)
                .unwrap_or("05_minimize_dfa")
        );
        return ExitCode::from(2);
    }
    let trace = match minimize::minimize(&args[1]) {
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
