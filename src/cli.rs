//! Command-line interface definition.

use clap::Parser;

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
}
