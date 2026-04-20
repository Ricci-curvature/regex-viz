//! Stage 1 CLI: regex string → build-trace JSON on stdout.
//!
//!   cargo run --example 01_build_nfa -- "a|b*c"
//!
//! Exit codes: 0 success, 1 parse error, 2 usage error.

use std::env;
use std::process::ExitCode;

use regex_viz::{nfa, parser};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <regex>", args.first().map(String::as_str).unwrap_or("01_build_nfa"));
        return ExitCode::from(2);
    }
    let src = &args[1];
    let ast = match parser::parse(src) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("parse error: {e}");
            return ExitCode::from(1);
        }
    };
    let trace = nfa::build_trace(&ast);
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
