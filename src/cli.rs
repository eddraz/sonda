//! Command-line interface definition.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "sonda",
    version,
    about = "PC hardware, OS, and driver scanner: one scan, structured JSON (contract v1)"
)]
pub struct Cli {
    /// Emit single-line JSON instead of pretty-printed.
    #[arg(long)]
    pub compact: bool,
    /// Render a human-readable summary instead of JSON.
    #[arg(long)]
    pub summary: bool,
    /// Optional subcommand; absent means a full scan.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Self-update the sonda binary from the latest GitHub release.
    Update {
        /// Emit single-line JSON instead of pretty-printed.
        #[arg(long)]
        compact: bool,
    },
}

impl Cli {
    /// Whether compact (single-line JSON) output was requested at any level.
    pub fn wants_compact(&self) -> bool {
        self.compact || matches!(&self.command, Some(Command::Update { compact }) if *compact)
    }
}
