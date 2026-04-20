//! Stage 2 CLI: regex + input → run-trace JSON on stdout.
//!
//!   cargo run --example 02_run_nfa -- "(a|b)*c" "aabc"
//!
//! Exit codes: 0 success, 1 error, 2 usage error.

use std::env;
use std::process::ExitCode;

use regex_viz::matcher;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: {} <regex> <input>",
            args.first().map(String::as_str).unwrap_or("02_run_nfa")
        );
        return ExitCode::from(2);
    }
    let trace = match matcher::run_trace(&args[1], &args[2]) {
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
