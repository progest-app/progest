//! `progest lint` — rules + accepts + sequence-drift report.
//!
//! Walks the project tree (or the filtered subset in `paths`) and
//! hands the result to `core::lint::lint_paths`. Emits the grouped
//! report as either a human-facing text list or a stable JSON wire
//! that downstream tools (Tauri panel, saved-search integrations,
//! CI gates) can consume.
//!
//! Exit code follows DSL §8.2:
//!
//! - `0` — no `strict` and no `evaluation_error` violations
//! - `1` — at least one `strict` or `evaluation_error`
//!
//! `--format json` always prints the JSON report to stdout,
//! regardless of exit code, so `progest lint --format json | jq ...`
//! works uniformly on clean and dirty trees.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use progest_core::accepts::{AliasCatalog, SchemaLoad, load_alias_catalog};
use progest_core::fs::{EntryKind, IgnoreRules, ScanEntry, Scanner, StdFileSystem};
use progest_core::lint::{LintOptions, LintReport, lint_paths};
use progest_core::meta::StdMetaStore;
use progest_core::naming::{CleanupConfig, extract_cleanup_config};
use progest_core::project::{ProjectDocument, ProjectRoot};
use progest_core::rules::{
    BUILTIN_COMPOUND_EXTS, CompiledRuleSet, RuleSetLayer, RuleSource, Severity, Violation,
    compile_ruleset, load_document,
};
use serde::Serialize;

use crate::output::{OutputFormat, emit_json};

pub struct LintArgs {
    pub paths: Vec<PathBuf>,
    pub format: OutputFormat,
    /// Retain rule traces for every evaluated file, not just violating
    /// ones. Produces a much larger JSON payload — only useful when
    /// authoring rules and wondering why a file matched the rule it did.
    pub explain: bool,
}

pub fn run(cwd: &Path, args: &LintArgs) -> Result<i32> {
    let root = ProjectRoot::discover(cwd).with_context(|| {
        format!(
            "could not find a Progest project at or above `{}`",
            cwd.display()
        )
    })?;

    let ruleset = load_ruleset(&root)?;
    let catalog = load_catalog(&root)?;
    let cleanup = load_cleanup(&root)?;

    let entries = collect_entries(&root, &args.paths)?;
    let paths: Vec<_> = entries.into_iter().map(|e| e.path).collect();

    let fs = StdFileSystem::new(root.root().to_path_buf());
    let store = StdMetaStore::new(fs);

    let opts = LintOptions {
        ruleset: &ruleset,
        alias_catalog: &catalog,
        compound_exts: BUILTIN_COMPOUND_EXTS,
        cleanup_cfg: &cleanup,
        explain: args.explain,
    };
    let report =
        lint_paths(store.filesystem(), &store, &paths, &opts).context("running lint pass")?;

    match args.format {
        OutputFormat::Text => emit_text(&report),
        OutputFormat::Json => emit_json(&report, "lint")?,
    }

    Ok(i32::from(report.fails_ci()))
}

// --- Config loaders --------------------------------------------------------

fn load_ruleset(root: &ProjectRoot) -> Result<CompiledRuleSet> {
    let path = root.rules_toml();
    if !path.exists() {
        return compile_ruleset(vec![]).context("compiling empty ruleset (should be infallible)");
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    let doc = load_document(&raw).with_context(|| format!("parsing `{}`", path.display()))?;
    // Surface loader warnings so rule authors see typos early.
    for w in &doc.warnings {
        eprintln!("warning: rules.toml: {w:?}");
    }
    let layer = RuleSetLayer {
        source: RuleSource::ProjectWide,
        base_dir: progest_core::fs::ProjectPath::root(),
        rules: doc.rules,
    };
    compile_ruleset(vec![layer]).with_context(|| format!("compiling `{}`", path.display()))
}

fn load_catalog(root: &ProjectRoot) -> Result<AliasCatalog> {
    let path = root.schema_toml();
    if !path.exists() {
        return Ok(AliasCatalog::builtin());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
    let SchemaLoad { catalog, warnings } =
        load_alias_catalog(&raw).with_context(|| format!("parsing `{}`", path.display()))?;
    for w in &warnings {
        eprintln!("warning: schema.toml: {w:?}");
    }
    Ok(catalog)
}

fn load_cleanup(root: &ProjectRoot) -> Result<CleanupConfig> {
    let raw = std::fs::read_to_string(root.project_toml())
        .with_context(|| format!("reading `{}`", root.project_toml().display()))?;
    let doc = ProjectDocument::from_toml_str(&raw)
        .with_context(|| format!("parsing `{}`", root.project_toml().display()))?;
    let (cfg, warns) = extract_cleanup_config(&doc.extra).context("extracting [cleanup]")?;
    for w in warns {
        eprintln!("warning: project.toml [cleanup]: {w:?}");
    }
    Ok(cfg)
}

fn collect_entries(root: &ProjectRoot, paths: &[PathBuf]) -> Result<Vec<ScanEntry>> {
    let fs = StdFileSystem::new(root.root().to_path_buf());
    let rules = IgnoreRules::load(&fs).with_context(|| {
        format!(
            "failed to load ignore rules from `{}`",
            root.root().display()
        )
    })?;
    let scanner = Scanner::new(root.root().to_path_buf(), rules);

    let mut out = Vec::new();
    for entry in scanner {
        let entry = entry.context("scan walk failed")?;
        if !matches!(entry.kind, EntryKind::File) {
            continue;
        }
        if !paths.is_empty() && !entry_matches_filter(&entry, paths, root.root()) {
            continue;
        }
        out.push(entry);
    }
    Ok(out)
}

fn entry_matches_filter(entry: &ScanEntry, paths: &[PathBuf], root: &Path) -> bool {
    paths.iter().any(|p| {
        let abs = if p.is_absolute() {
            p.clone()
        } else {
            root.join(p)
        };
        let Ok(rel) = abs.strip_prefix(root) else {
            return false;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        entry.path.as_str().starts_with(rel_str.as_str())
    })
}

// --- Emit ------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct TextBucket<'a> {
    title: &'static str,
    rows: &'a [Violation],
}

fn emit_text(report: &LintReport) {
    let buckets = [
        TextBucket {
            title: "naming",
            rows: &report.naming,
        },
        TextBucket {
            title: "placement",
            rows: &report.placement,
        },
        TextBucket {
            title: "sequence",
            rows: &report.sequence,
        },
    ];

    let mut any = false;
    for b in &buckets {
        if b.rows.is_empty() {
            continue;
        }
        any = true;
        println!("[{}]", b.title);
        for v in b.rows {
            println!(
                "  {} ({}) [{}]",
                v.path.as_str(),
                v.rule_id.as_str(),
                severity_label(v.severity),
            );
            println!("    {}", v.reason);
            if !v.suggested_names.is_empty() {
                println!("    suggest: {}", v.suggested_names.join(", "));
            }
        }
        println!();
    }

    if !any {
        println!("(clean)");
    }

    let s = &report.summary;
    println!(
        "Summary: {} scanned — {} naming / {} placement / {} sequence — {} strict, {} eval-error, {} warn, {} hint",
        s.scanned,
        s.naming_count,
        s.placement_count,
        s.sequence_count,
        s.strict_count,
        s.evaluation_error_count,
        s.warn_count,
        s.hint_count,
    );
}

fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Strict => "strict",
        Severity::Warn => "warn",
        Severity::Hint => "hint",
        Severity::EvaluationError => "eval-error",
    }
}
