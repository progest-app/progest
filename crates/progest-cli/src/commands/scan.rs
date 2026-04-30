//! `progest scan` — reconcile the project tree against the index.

use std::path::Path;

use anyhow::{Context, Result};
use progest_core::fs::StdFileSystem;
use progest_core::index::SqliteIndex;
use progest_core::meta::StdMetaStore;
use progest_core::reconcile::{Reconciler, ScanReport};
use progest_core::thumbnail::{self, DEFAULT_CACHE_MAX_BYTES, ThumbnailCache};

use crate::context::discover_root;

/// Run `progest scan` starting the discovery walk from `cwd`.
pub fn run(cwd: &Path) -> Result<()> {
    let root = discover_root(cwd)?;
    let report = scan_with_root(root.root(), &root)?;
    print_report(&report);

    let thumb_report = thumbnail::generate_for_outcomes(
        &report.outcomes,
        &cache_for(&root),
        root.root(),
        &index_for(&root)?,
    );
    if thumb_report.generated > 0 || thumb_report.skipped > 0 {
        println!(
            "Thumbnails: {} generated, {} cached, {} skipped",
            thumb_report.generated, thumb_report.cached, thumb_report.skipped
        );
    }

    Ok(())
}

/// Shared scan routine used by both `scan` and `doctor` so the two never
/// disagree about what "reconciled" means.
pub(crate) fn scan(cwd: &Path) -> Result<ScanReport> {
    let root = discover_root(cwd)?;
    scan_with_root(root.root(), &root)
}

fn scan_with_root(
    _root_path: &Path,
    root: &progest_core::project::ProjectRoot,
) -> Result<ScanReport> {
    let fs = StdFileSystem::new(root.root().to_path_buf());
    let meta = StdMetaStore::new(fs.clone());
    let index = SqliteIndex::open(&root.index_db())
        .with_context(|| format!("failed to open index at `{}`", root.index_db().display()))?;
    let reconciler = Reconciler::new(&fs, &meta, &index);
    reconciler.full_scan().context("reconcile full scan failed")
}

fn cache_for(root: &progest_core::project::ProjectRoot) -> ThumbnailCache {
    ThumbnailCache::new(root.thumbs_dir(), DEFAULT_CACHE_MAX_BYTES)
}

fn index_for(root: &progest_core::project::ProjectRoot) -> Result<SqliteIndex> {
    SqliteIndex::open(&root.index_db())
        .with_context(|| format!("failed to open index at `{}`", root.index_db().display()))
}

fn print_report(report: &ScanReport) {
    println!(
        "Scanned: {} added, {} updated, {} unchanged, {} removed",
        report.added(),
        report.updated(),
        report.unchanged(),
        report.removed(),
    );
    if !report.orphan_metas.is_empty() {
        println!(
            "Warning: {} orphan `.meta` file(s) detected (run `progest doctor` for details).",
            report.orphan_metas.len(),
        );
    }
}
