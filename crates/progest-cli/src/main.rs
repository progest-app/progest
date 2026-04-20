//! Progest CLI entry point.
//!
//! The CLI is a first-class interface alongside the GUI. Every subcommand
//! should be backed by a `progest-core` API with identical behaviour.
//! See `docs/REQUIREMENTS.md` §3.9 for the full command surface.

#![allow(clippy::todo)] // scaffold: handlers populated in M1+.

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Naming-rule-first file management for creative projects.
#[derive(Debug, Parser)]
#[command(name = "progest", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize a new Progest project in the current directory.
    Init,
    /// Walk the project and (re)build the index.
    Scan,
    /// Report integrity issues (orphan meta, UUID clashes, drift).
    Doctor,
    /// Check files against naming rules.
    Lint,
    /// Search files using the Progest query DSL.
    Search {
        /// The query string (e.g. `tag:character type:psd is:violation`).
        query: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Init => todo!("M1: initialize .progest/ layout"),
        Command::Scan => todo!("M1: scan project and populate index"),
        Command::Doctor => todo!("M1: integrity report"),
        Command::Lint => todo!("M2: rule engine lint report"),
        Command::Search { query: _ } => todo!("M3: DSL parser + FTS5 query"),
    }
}
