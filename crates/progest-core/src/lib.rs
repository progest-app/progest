//! Progest core: domain logic for metadata, naming rules, indexing, and search.
//!
//! This crate is the authoritative home for all business logic. UI, CLI, and
//! IPC layers are thin wrappers over this library. See the project
//! `docs/IMPLEMENTATION_PLAN.md` for the full module layout.
//!
//! The crate is intentionally empty at M0. Subsequent milestones will populate
//! `fs`, `meta`, `identity`, `rules`, `index`, `search`, `watch`, `reconcile`,
//! `thumbnail`, `template`, `ai`, `history`, `rename`, and `doctor` modules.

/// The crate version, synced with the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
