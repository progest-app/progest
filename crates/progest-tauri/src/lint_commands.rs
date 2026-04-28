//! `lint_run` IPC — recompute the project's lint report and write the
//! resulting violations into the index.
//!
//! Used by the directory inspector after an `[accepts]` edit so the
//! `is:violation` / `is:misplaced` queries (and the placement badges
//! that derive from them) reflect the new declaration. Without this
//! refresh, badges would stay stale until the user shells out to
//! `progest lint` themselves — exactly the gap the M3 #9 inspector is
//! supposed to close.
//!
//! Mirrors `progest-cli`'s lint command: it loads `rules.toml`,
//! `schema.toml [alias]`, and `project.toml [cleanup]`, walks the
//! project's indexed paths, runs `core::lint::lint_paths`, and persists
//! the report through `write_to_index`.

use progest_core::accepts::{AliasCatalog, SchemaLoad, load_alias_catalog};
use progest_core::fs::ProjectPath;
use progest_core::index::Index;
use progest_core::lint::{LintOptions, lint_paths, write_to_index};
use progest_core::meta::StdMetaStore;
use progest_core::naming::{CleanupConfig, extract_cleanup_config};
use progest_core::project::ProjectDocument;
use progest_core::rules::{
    BUILTIN_COMPOUND_EXTS, CompiledRuleSet, RuleSetLayer, RuleSource, compile_ruleset,
    load_document,
};
use serde::Serialize;
use tauri::State;

use crate::commands::no_project_error;
use crate::state::{AppState, ProjectContext};

#[derive(Debug, Clone, Serialize)]
pub struct LintRunResponse {
    pub naming: usize,
    pub placement: usize,
    pub sequence: usize,
    pub scanned: usize,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn lint_run(state: State<'_, AppState>) -> Result<LintRunResponse, String> {
    let guard = state.project.lock().expect("project mutex poisoned");
    let ctx = guard.as_ref().ok_or_else(no_project_error)?;

    let ruleset = load_ruleset(ctx)?;
    let catalog = load_alias_catalog_for_ctx(ctx);
    let cleanup = load_cleanup(ctx)?;

    // Walk the index for the path set rather than re-walking the FS —
    // the inspector triggers this synchronously on save, and reusing
    // the indexed list keeps the round-trip predictable. A subsequent
    // `progest scan` will catch any FS additions the index hasn't seen
    // yet.
    let rows = ctx
        .index
        .list_files()
        .map_err(|e| format!("list indexed files: {e}"))?;
    let paths: Vec<ProjectPath> = rows.iter().map(|r| r.path.clone()).collect();

    let store = StdMetaStore::new(ctx.fs.clone());
    let opts = LintOptions {
        ruleset: &ruleset,
        alias_catalog: &catalog,
        compound_exts: BUILTIN_COMPOUND_EXTS,
        cleanup_cfg: &cleanup,
        explain: false,
    };
    let report = lint_paths(store.filesystem(), &store, &paths, &opts)
        .map_err(|e| format!("running lint pass: {e}"))?;

    let visited: Vec<_> = rows.into_iter().map(|r| r.file_id).collect();
    write_to_index(&ctx.index, &visited, &report)
        .map_err(|e| format!("write violations to index: {e}"))?;

    Ok(LintRunResponse {
        naming: report.naming.len(),
        placement: report.placement.len(),
        sequence: report.sequence.len(),
        scanned: paths.len(),
    })
}

// --- helpers ---------------------------------------------------------------

fn load_ruleset(ctx: &ProjectContext) -> Result<CompiledRuleSet, String> {
    let path = ctx.root.rules_toml();
    if !path.exists() {
        return compile_ruleset(vec![]).map_err(|e| format!("compile empty ruleset: {e}"));
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let doc = load_document(&raw).map_err(|e| format!("parse rules.toml: {e}"))?;
    let layer = RuleSetLayer {
        source: RuleSource::ProjectWide,
        base_dir: ProjectPath::root(),
        rules: doc.rules,
    };
    compile_ruleset(vec![layer]).map_err(|e| format!("compile ruleset: {e}"))
}

fn load_alias_catalog_for_ctx(ctx: &ProjectContext) -> AliasCatalog {
    let path = ctx.root.schema_toml();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return AliasCatalog::builtin();
    };
    match load_alias_catalog(&text) {
        Ok(SchemaLoad { catalog, .. }) => catalog,
        Err(_) => AliasCatalog::builtin(),
    }
}

fn load_cleanup(ctx: &ProjectContext) -> Result<CleanupConfig, String> {
    let path = ctx.root.project_toml();
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read `{}`: {e}", path.display()))?;
    let doc =
        ProjectDocument::from_toml_str(&raw).map_err(|e| format!("parse project.toml: {e}"))?;
    let (cfg, _warns) =
        extract_cleanup_config(&doc.extra).map_err(|e| format!("read [cleanup]: {e}"))?;
    Ok(cfg)
}
