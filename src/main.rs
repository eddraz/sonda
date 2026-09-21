//! sonda: PC hardware, OS, and driver scanner as JSON (contract v1).

mod cli;
mod model;
mod probe;
mod run;
mod shell;
mod summary;
mod timefmt;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser as _;

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    if args.summary {
        return print_summary(&run::run_scan());
    }

    serialize(run::run_scan(), args.compact)
}

fn print_summary(output: &model::Output) -> ExitCode {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{}", summary::render(output));
    let _ = lock.flush();
    ExitCode::SUCCESS
}

fn serialize(output: model::Output, compact: bool) -> ExitCode {
    let json = if compact {
        serde_json::to_string(&output)
    } else {
        serde_json::to_string_pretty(&output)
    };
    match json {
        Ok(body) => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            let _ = writeln!(lock, "{body}");
            let _ = lock.flush();
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sonda: failed to serialize output: {e}");
            ExitCode::FAILURE
        }
    }
}
