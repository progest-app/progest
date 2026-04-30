//! `progest import` — bring external files into the project.
//!
//! Supports copy (default) and move (`--move`), preview-only
//! (`--dry-run`) and apply modes, sequence detection for batches,
//! and both JSON and text output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use progest_core::fs::{ProjectPath, StdFileSystem};
use progest_core::import::{self, Import, ImportMode, ImportRequest, build_preview};
use progest_core::meta::StdMetaStore;

use crate::context;
use crate::output::OutputFormat;

pub struct ImportArgs {
    pub files: Vec<PathBuf>,
    pub dest: Option<String>,
    pub is_move: bool,
    pub dry_run: bool,
    pub format: OutputFormat,
}

pub fn run(cwd: &Path, args: &ImportArgs) -> Result<i32> {
    if args.files.is_empty() {
        bail!("no files specified");
    }

    let root = context::discover_root(cwd)?;
    let index = context::open_index(&root)?;
    let history = context::open_history(&root)?;
    let fs = StdFileSystem::new(root.root().to_path_buf());
    let meta_store = StdMetaStore::new(fs.clone());

    let mode = if args.is_move {
        ImportMode::Move
    } else {
        ImportMode::Copy
    };

    let project_root = root.root();
    let requests = build_requests(&args.files, args.dest.as_deref(), mode, project_root)?;

    let preview = build_preview(&requests, &fs, project_root);

    if args.dry_run {
        emit_preview(&preview, args.format);
        let has_conflicts = !preview.is_clean();
        return Ok(i32::from(has_conflicts));
    }

    if !preview.is_clean() {
        emit_preview(&preview, args.format);
        eprintln!(
            "error: {} conflict(s) detected; resolve before importing (or use --dry-run to inspect)",
            preview.conflicting_ops().count()
        );
        return Ok(1);
    }

    let driver = Import::new(&fs, &meta_store, &index, &history, project_root);
    let outcome = driver.apply(&preview).context("import apply failed")?;

    emit_outcome(&outcome, args.format);
    Ok(0)
}

fn build_requests(
    files: &[PathBuf],
    dest: Option<&str>,
    mode: ImportMode,
    _project_root: &Path,
) -> Result<Vec<ImportRequest>> {
    let mut requests = Vec::with_capacity(files.len());

    for file in files {
        let source = if file.is_absolute() {
            file.clone()
        } else {
            std::env::current_dir()?.join(file)
        };

        let dest_path = if let Some(d) = dest {
            let dir = ProjectPath::new(d).with_context(|| format!("invalid destination `{d}`"))?;
            let filename = source
                .file_name()
                .with_context(|| format!("cannot determine filename for `{}`", source.display()))?
                .to_string_lossy();
            dir.join(&*filename)?
        } else {
            let filename = source
                .file_name()
                .with_context(|| format!("cannot determine filename for `{}`", source.display()))?
                .to_string_lossy();
            ProjectPath::new(&*filename)?
        };

        requests.push(ImportRequest {
            source,
            dest: dest_path,
            mode,
            group_id: None,
        });
    }

    Ok(requests)
}

fn emit_preview(preview: &import::ImportPreview, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "ops": preview.ops,
                "clean": preview.is_clean(),
                "conflict_count": preview.conflicting_ops().count(),
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        OutputFormat::Text => {
            for op in &preview.ops {
                if op.is_clean() {
                    let verb = match op.mode {
                        ImportMode::Copy => "copy",
                        ImportMode::Move => "move",
                    };
                    println!("  {verb} {} → {}", op.source, op.dest);
                } else {
                    println!("  CONFLICT {} → {}", op.source, op.dest);
                    for c in &op.conflicts {
                        println!("    {c:?}");
                    }
                }
            }
            let total = preview.ops.len();
            let conflicts = preview.conflicting_ops().count();
            println!(
                "\n{total} file(s), {conflicts} conflict(s), {} ready",
                total - conflicts
            );
        }
    }
}

fn emit_outcome(outcome: &import::ImportOutcome, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "batch_id": outcome.batch_id,
                "group_id": outcome.group_id,
                "imported": outcome.imported,
                "warnings": outcome.warnings,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        OutputFormat::Text => {
            for f in &outcome.imported {
                let verb = match f.mode {
                    ImportMode::Copy => "copied",
                    ImportMode::Move => "moved",
                };
                println!("  {verb} {} → {}", f.source, f.dest);
            }
            if !outcome.warnings.is_empty() {
                eprintln!("{} warning(s):", outcome.warnings.len());
                for w in &outcome.warnings {
                    eprintln!("  {w:?}");
                }
            }
            println!(
                "\n{} file(s) imported{}",
                outcome.imported.len(),
                outcome
                    .group_id
                    .as_ref()
                    .map(|g| format!(" (group {g})"))
                    .unwrap_or_default()
            );
        }
    }
}
