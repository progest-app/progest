//! `progest delete` — move a project file to the OS trash.
//!
//! The file and its `.meta` sidecar (if present) are moved to the
//! OS trash via the `trash` crate. The index entry is removed so the
//! file disappears from search / views immediately.

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result, bail};
use progest_core::delete::{apply_delete, preview_delete};
use progest_core::fs::ProjectPath;

use crate::context;
use crate::output::OutputFormat;

pub struct DeleteArgs {
    pub path: String,
    pub dry_run: bool,
    pub force: bool,
    pub format: OutputFormat,
}

pub fn run(cwd: &Path, args: &DeleteArgs) -> Result<i32> {
    let root = context::discover_root(cwd)?;
    let index = context::open_index(&root)?;

    let project_path =
        ProjectPath::new(&args.path).with_context(|| format!("invalid path `{}`", args.path))?;

    let preview =
        preview_delete(&index, root.root(), &project_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    if args.dry_run {
        emit_preview(&preview, args.format);
        return Ok(0);
    }

    if !args.force {
        let label = if preview.has_sidecar {
            format!("{} (+ .meta sidecar)", preview.path)
        } else {
            preview.path.as_str().to_owned()
        };
        eprintln!("will trash: {label}");
        eprintln!("use --force to skip this confirmation, or --dry-run to preview");
        if std::io::stdin().is_terminal() {
            eprint!("proceed? [y/N] ");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            if !line.trim().eq_ignore_ascii_case("y") {
                eprintln!("aborted");
                return Ok(1);
            }
        } else {
            bail!("non-interactive mode requires --force");
        }
    }

    let outcome =
        apply_delete(&index, root.root(), &project_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    emit_outcome(&outcome, args.format);
    Ok(0)
}

fn emit_preview(preview: &progest_core::delete::DeletePreview, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "path": preview.path,
                "file_id": preview.file_id.to_string(),
                "has_sidecar": preview.has_sidecar,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        OutputFormat::Text => {
            println!("  {} (file_id: {})", preview.path, preview.file_id);
            if preview.has_sidecar {
                println!("  + .meta sidecar will also be trashed");
            }
        }
    }
}

fn emit_outcome(outcome: &progest_core::delete::DeleteOutcome, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "path": outcome.path,
                "file_id": outcome.file_id.to_string(),
                "sidecar_trashed": outcome.sidecar_trashed,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        OutputFormat::Text => {
            println!("  trashed {}", outcome.path);
            if outcome.sidecar_trashed {
                println!("  + .meta sidecar also trashed");
            }
        }
    }
}
