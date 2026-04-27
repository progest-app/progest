//! Search-query recent-history log (`docs/M3_HANDOFF.md` §3.3).
//!
//! Backed by `.progest/local/search-history.json` — machine-local
//! (gitignored) so each operator builds up their own recent queries.
//! The command palette uses it as the empty-input candidate list.
//!
//! Retention: newest [`MAX_ENTRIES`] entries kept; older ones are
//! dropped on every [`append`]. Duplicate queries are de-duplicated by
//! moving the existing entry to the front so the most-recent timestamp
//! always wins.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fs::{FileSystem, FsError, ProjectPath};

/// Schema version stamped into the JSON file. Bump on incompatible
/// shape changes; readers reject unknown versions to avoid surprising
/// truncation when an older binary touches a newer file.
pub const HISTORY_SCHEMA_VERSION: u32 = 1;

/// Hard cap on retained entries. Trimmed on every [`append`].
pub const MAX_ENTRIES: usize = 100;

/// One recorded query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The DSL query string exactly as the user submitted it.
    pub query: String,
    /// RFC 3339 timestamp (UTC) of the most recent submission.
    pub ts: DateTime<Utc>,
}

/// Top-level `search-history.json` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub entries: Vec<HistoryEntry>,
}

impl Default for HistoryDocument {
    fn default() -> Self {
        Self {
            schema_version: HISTORY_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("search-history.json not present")]
    NotFound,
    #[error("read/write search-history.json: {0}")]
    Fs(#[from] FsError),
    #[error("parse search-history.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "unsupported search-history.json schema_version {found}; this build understands {expected}"
    )]
    UnsupportedSchema { found: u32, expected: u32 },
}

/// Load the history document at `path`. A missing file surfaces as
/// [`HistoryError::NotFound`] so callers can choose to default to
/// empty (the typical UI flow).
pub fn load(fs: &dyn FileSystem, path: &ProjectPath) -> Result<HistoryDocument, HistoryError> {
    let bytes = match fs.read(path) {
        Ok(b) => b,
        Err(FsError::NotFound(_)) => return Err(HistoryError::NotFound),
        Err(e) => return Err(HistoryError::Fs(e)),
    };
    let doc: HistoryDocument = serde_json::from_slice(&bytes)?;
    if doc.schema_version != HISTORY_SCHEMA_VERSION {
        return Err(HistoryError::UnsupportedSchema {
            found: doc.schema_version,
            expected: HISTORY_SCHEMA_VERSION,
        });
    }
    Ok(doc)
}

/// Persist `doc` to `path` atomically.
pub fn save(
    fs: &dyn FileSystem,
    path: &ProjectPath,
    doc: &HistoryDocument,
) -> Result<(), HistoryError> {
    let text = serde_json::to_vec_pretty(doc)?;
    fs.write_atomic(path, &text)?;
    Ok(())
}

/// Record `query` as the most-recent entry. Empty / whitespace-only
/// queries are silently ignored so the UI can call this on every
/// submit without filtering. If `query` already appears in `entries`
/// (exact string match), the existing entry is removed before
/// prepending — this keeps the log de-duplicated while letting the
/// timestamp track the latest submission.
pub fn append(doc: &mut HistoryDocument, query: &str, ts: DateTime<Utc>) {
    if query.trim().is_empty() {
        return;
    }
    doc.entries.retain(|e| e.query != query);
    doc.entries.insert(
        0,
        HistoryEntry {
            query: query.to_string(),
            ts,
        },
    );
    if doc.entries.len() > MAX_ENTRIES {
        doc.entries.truncate(MAX_ENTRIES);
    }
}

/// Drop every recorded entry. Schema version is preserved so the
/// next [`save`] still produces a valid file.
pub fn clear(doc: &mut HistoryDocument) {
    doc.entries.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::mem::MemFileSystem;
    use chrono::TimeZone;

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
    }

    fn path() -> ProjectPath {
        ProjectPath::new(".progest/local/search-history.json").unwrap()
    }

    #[test]
    fn missing_file_returns_not_found() {
        let fs = MemFileSystem::new();
        let err = load(&fs, &path()).unwrap_err();
        assert!(matches!(err, HistoryError::NotFound));
    }

    #[test]
    fn round_trip_save_and_load() {
        let fs = MemFileSystem::new();
        let mut doc = HistoryDocument::default();
        append(&mut doc, "tag:wip", ts(0));
        append(&mut doc, "type:psd", ts(1));
        save(&fs, &path(), &doc).unwrap();

        let loaded = load(&fs, &path()).unwrap();
        assert_eq!(loaded.schema_version, HISTORY_SCHEMA_VERSION);
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].query, "type:psd");
        assert_eq!(loaded.entries[1].query, "tag:wip");
    }

    #[test]
    fn append_prepends_in_recency_order() {
        let mut doc = HistoryDocument::default();
        append(&mut doc, "a", ts(0));
        append(&mut doc, "b", ts(1));
        append(&mut doc, "c", ts(2));
        let queries: Vec<&str> = doc.entries.iter().map(|e| e.query.as_str()).collect();
        assert_eq!(queries, vec!["c", "b", "a"]);
    }

    #[test]
    fn append_dedups_by_moving_existing_to_front() {
        let mut doc = HistoryDocument::default();
        append(&mut doc, "a", ts(0));
        append(&mut doc, "b", ts(1));
        append(&mut doc, "a", ts(2));
        assert_eq!(doc.entries.len(), 2);
        assert_eq!(doc.entries[0].query, "a");
        assert_eq!(doc.entries[0].ts, ts(2));
        assert_eq!(doc.entries[1].query, "b");
    }

    #[test]
    fn append_ignores_empty_query() {
        let mut doc = HistoryDocument::default();
        append(&mut doc, "", ts(0));
        append(&mut doc, "   ", ts(1));
        assert!(doc.entries.is_empty());
    }

    #[test]
    fn append_trims_to_max_entries() {
        let mut doc = HistoryDocument::default();
        for i in 0..(MAX_ENTRIES + 5) {
            append(&mut doc, &format!("q{i}"), ts(i64::try_from(i).unwrap()));
        }
        assert_eq!(doc.entries.len(), MAX_ENTRIES);
        // Most-recent first: q104 down to q5 (q0..q4 dropped).
        assert_eq!(doc.entries[0].query, format!("q{}", MAX_ENTRIES + 4));
        assert_eq!(doc.entries[MAX_ENTRIES - 1].query, "q5");
    }

    #[test]
    fn clear_resets_entries_but_keeps_schema_version() {
        let mut doc = HistoryDocument::default();
        append(&mut doc, "tag:wip", ts(0));
        clear(&mut doc);
        assert!(doc.entries.is_empty());
        assert_eq!(doc.schema_version, HISTORY_SCHEMA_VERSION);
    }

    #[test]
    fn schema_version_mismatch_rejected() {
        let fs = MemFileSystem::new();
        let body = br#"{"schema_version":9,"entries":[]}"#;
        fs.write_atomic(&path(), body).unwrap();
        let err = load(&fs, &path()).unwrap_err();
        assert!(matches!(err, HistoryError::UnsupportedSchema { .. }));
    }

    #[test]
    fn malformed_json_returns_json_error() {
        let fs = MemFileSystem::new();
        fs.write_atomic(&path(), b"{not json").unwrap();
        let err = load(&fs, &path()).unwrap_err();
        assert!(matches!(err, HistoryError::Json(_)));
    }

    #[test]
    fn save_overwrites_existing_atomically() {
        let fs = MemFileSystem::new();
        let mut doc = HistoryDocument::default();
        append(&mut doc, "first", ts(0));
        save(&fs, &path(), &doc).unwrap();
        clear(&mut doc);
        append(&mut doc, "second", ts(1));
        save(&fs, &path(), &doc).unwrap();
        let loaded = load(&fs, &path()).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].query, "second");
    }
}
