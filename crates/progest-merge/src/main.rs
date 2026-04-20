//! Git merge driver for Progest `.meta` files.
//!
//! Configured in `.gitattributes`:
//!
//! ```text
//! *.meta merge=progest-meta
//! ```
//!
//! and `.git/config` (or `.git/info/attributes`):
//!
//! ```text
//! [merge "progest-meta"]
//!     name = Progest meta merge driver
//!     driver = progest-merge %O %A %B
//! ```
//!
//! Git passes three temp files: the common ancestor (`%O`), the current
//! branch version (`%A`), and the other branch version (`%B`). The driver
//! must write the merged result back to the `%A` path and exit 0 on clean
//! merge, non-zero on conflict.
//!
//! Merge semantics are defined in `docs/REQUIREMENTS.md` §6.2.

#![allow(clippy::todo)] // scaffold: real merger lands in M2+.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "progest-merge", version, about, long_about = None)]
struct Args {
    /// Common ancestor version (`%O` from git).
    ancestor: PathBuf,
    /// Current branch version (`%A`); the merged result is written back here.
    ours: PathBuf,
    /// Other branch version (`%B`).
    theirs: PathBuf,
}

fn main() -> Result<()> {
    let _args = Args::parse();
    todo!("M2+: three-way merge of Progest .meta TOML (tags union, notes concat, custom key-wise)")
}
