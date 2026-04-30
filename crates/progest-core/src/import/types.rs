//! Wire types for `core::import`.

use serde::Serialize;

use crate::fs::ProjectPath;

/// How the source file should be brought into the project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// Copy the source, leaving the original intact (default).
    #[default]
    Copy,
    /// Move the source (destructive — opt-in via `--move`).
    Move,
}

/// A single import request, before conflict detection.
#[derive(Debug, Clone)]
pub struct ImportRequest {
    /// Absolute or project-external path to the source file.
    pub source: std::path::PathBuf,
    /// Destination path inside the project (project-relative).
    pub dest: ProjectPath,
    /// Copy or move.
    pub mode: ImportMode,
    /// Optional caller-supplied group (e.g. from sequence batching).
    pub group_id: Option<String>,
}

/// A single import op after conflict detection (may carry conflicts).
#[derive(Debug, Clone, Serialize)]
pub struct ImportOp {
    pub source: String,
    pub dest: ProjectPath,
    pub mode: ImportMode,
    pub group_id: Option<String>,
    pub conflicts: Vec<ImportConflict>,
}

impl ImportOp {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }
}

/// Why an import op cannot proceed without user intervention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportConflict {
    /// Destination already exists in the project.
    DestExists { existing_path: ProjectPath },
    /// Source file is missing or inaccessible.
    SourceMissing { reason: String },
    /// Source is already inside the project (use `rename` instead).
    SourceIsProject { project_path: ProjectPath },
    /// Destination dir doesn't accept this file's extension.
    PlacementMismatch {
        expected_exts: Vec<String>,
        suggestion: Option<ProjectPath>,
    },
}

/// Result of `build_preview`.
#[derive(Debug)]
pub struct ImportPreview {
    pub ops: Vec<ImportOp>,
}

impl ImportPreview {
    /// True when every op is conflict-free.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.ops.iter().all(ImportOp::is_clean)
    }

    /// Iterator over ops that carry at least one conflict.
    pub fn conflicting_ops(&self) -> impl Iterator<Item = &ImportOp> {
        self.ops.iter().filter(|op| !op.is_clean())
    }

    /// Iterator over ops that are ready to apply.
    pub fn clean_ops(&self) -> impl Iterator<Item = &ImportOp> {
        self.ops.iter().filter(|op| op.is_clean())
    }
}
