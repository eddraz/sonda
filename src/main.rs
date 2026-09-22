//! sonda: PC hardware, OS, and driver scanner as JSON (contract v1).

mod cli;
mod model;
mod probe;
mod run;
mod shell;
mod summary;
mod timefmt;
mod update;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser as _;

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    let compact = args.wants_compact();

    if let Some(cli::Command::Update { check, .. }) = args.command {
        if args.summary {
            eprintln!("sonda: --summary is not valid with update");
            return ExitCode::from(2);
        }
        let result = if check {
            update::check_update()
        } else {
            update::run_update()
        };
        return match result {
            Ok(report) => print_json(&report, compact),
            Err(report) => {
                print_json(&report, compact);
                ExitCode::FAILURE
            }
        };
    }

    if args.summary {
        return print_summary(&run::run_scan());
    }

    print_json(&run::run_scan(), compact)
}

fn print_summary(output: &model::Output) -> ExitCode {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{}", summary::render(output));
    let _ = lock.flush();
    ExitCode::SUCCESS
}

fn print_json<T: serde::Serialize + ?Sized>(value: &T, compact: bool) -> ExitCode {
    let json = if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
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
