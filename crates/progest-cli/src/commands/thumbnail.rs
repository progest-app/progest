use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};

use progest_core::index::Index;
use progest_core::thumbnail::{
    self, CleanReport, DEFAULT_CACHE_MAX_BYTES, GenerateBatchReport, ThumbnailCache,
    ThumbnailResult,
};

use crate::context;
use crate::output::{self, OutputFormat};

pub struct ThumbnailGenerateArgs {
    pub paths: Vec<std::path::PathBuf>,
    pub format: OutputFormat,
    pub force: bool,
    pub size: u32,
}

pub struct ThumbnailCleanArgs {
    pub format: OutputFormat,
}

pub fn run_generate(cwd: &Path, args: &ThumbnailGenerateArgs) -> Result<i32> {
    let root = context::discover_root(cwd)?;
    let index = context::open_index(&root)?;
    let cache = ThumbnailCache::new(root.thumbs_dir(), DEFAULT_CACHE_MAX_BYTES);

    let requests = if args.paths.is_empty() {
        thumbnail::requests_from_index(&index, root.root(), args.size)
    } else {
        let mut reqs = Vec::new();
        for p in &args.paths {
            let abs = if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            };
            let rel = abs
                .strip_prefix(root.root())
                .with_context(|| format!("{} is not inside the project", p.display()))?;
            let proj_path = progest_core::fs::ProjectPath::new(
                rel.to_str()
                    .with_context(|| format!("non-UTF-8 path: {}", rel.display()))?,
            )?;
            if let Some(row) = index.get_file_by_path(&proj_path)? {
                reqs.push(thumbnail::ThumbnailRequest {
                    abs_path: abs,
                    path: proj_path,
                    file_id: row.file_id,
                    fingerprint: row.fingerprint,
                    size: args.size,
                });
            } else {
                eprintln!(
                    "warning: {} not in index (run `progest scan` first)",
                    p.display()
                );
            }
        }
        reqs
    };

    if requests.is_empty() {
        match args.format {
            OutputFormat::Json => {
                output::emit_json(&GenerateBatchReport::default(), "thumbnail generate")?;
            }
            OutputFormat::Text => println!("No files to generate thumbnails for."),
        }
        return Ok(0);
    }

    let report = thumbnail::generate_batch(&requests, &cache, args.force);
    emit_generate_report(&report, args.format)?;
    Ok(0)
}

pub fn run_clean(cwd: &Path, args: &ThumbnailCleanArgs) -> Result<i32> {
    let root = context::discover_root(cwd)?;
    let index = context::open_index(&root)?;
    let cache = ThumbnailCache::new(root.thumbs_dir(), DEFAULT_CACHE_MAX_BYTES);

    let rows = index.list_files()?;
    let live_ids: HashSet<_> = rows.into_iter().map(|r| r.file_id).collect();

    let report = cache
        .clean(&live_ids)
        .context("thumbnail cache clean failed")?;
    emit_clean_report(&report, args.format)?;
    Ok(0)
}

fn emit_generate_report(report: &GenerateBatchReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => output::emit_json(report, "thumbnail generate"),
        OutputFormat::Text => {
            for result in &report.results {
                match result {
                    ThumbnailResult::Generated { path, bytes, .. } => {
                        println!("  generated  {path}  ({bytes} bytes)");
                    }
                    ThumbnailResult::Cached { path, .. } => {
                        println!("  cached     {path}");
                    }
                    ThumbnailResult::Skipped { path, reason } => {
                        let msg = match reason {
                            thumbnail::SkipReason::UnsupportedFormat { ext } => {
                                format!("unsupported format: {ext}")
                            }
                            thumbnail::SkipReason::FfmpegNotFound => "ffmpeg not found".to_owned(),
                            thumbnail::SkipReason::GenerationFailed { message } => message.clone(),
                            thumbnail::SkipReason::SourceMissing => "source missing".to_owned(),
                        };
                        println!("  skipped    {path}  ({msg})");
                    }
                }
            }
            println!(
                "\n{} generated, {} cached, {} skipped",
                report.generated, report.cached, report.skipped
            );
            Ok(())
        }
    }
}

fn emit_clean_report(report: &CleanReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => output::emit_json(report, "thumbnail clean"),
        OutputFormat::Text => {
            println!(
                "Cleaned: {} orphans removed, {} LRU evicted, {} bytes freed",
                report.orphans_removed, report.lru_evicted, report.bytes_freed
            );
            Ok(())
        }
    }
}
