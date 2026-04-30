//! End-to-end tests for `core::import`.
//!
//! Exercises the full import flow: external file → preview → apply →
//! .meta sidecar created → index row registered → history entry appended.

mod support;

use std::io::Write;

use progest_core::fs::StdFileSystem;
use progest_core::history::{OpKind, SqliteStore, Store};
use progest_core::import::{Import, ImportMode, ImportRequest, build_preview};
use progest_core::index::{Index, SqliteIndex};
use progest_core::meta::StdMetaStore;
use tempfile::TempDir;

use support::p;

struct Harness {
    project_dir: TempDir,
    source_dir: TempDir,
    fs: StdFileSystem,
    meta_store: StdMetaStore<StdFileSystem>,
    index: SqliteIndex,
    history: SqliteStore,
}

impl Harness {
    fn new() -> Self {
        let project_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();

        std::fs::create_dir_all(project_dir.path().join(".progest/local/staging")).unwrap();

        let fs = StdFileSystem::new(project_dir.path().to_path_buf());
        let meta_store = StdMetaStore::new(fs.clone());
        let index = SqliteIndex::open_in_memory().unwrap();
        let history_path = project_dir.path().join(".progest/local/history.db");
        let history = SqliteStore::open(&history_path).unwrap();

        Self {
            project_dir,
            source_dir,
            fs,
            meta_store,
            index,
            history,
        }
    }

    fn write_source(&self, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = self.source_dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    fn import_driver(&self) -> Import<'_> {
        Import::new(
            &self.fs,
            &self.meta_store,
            &self.index,
            &self.history,
            self.project_dir.path(),
        )
    }

    fn project_file_exists(&self, rel: &str) -> bool {
        self.project_dir.path().join(rel).exists()
    }

    fn project_file_content(&self, rel: &str) -> Vec<u8> {
        std::fs::read(self.project_dir.path().join(rel)).unwrap()
    }
}

// --- Tests -------------------------------------------------------------------

#[test]
fn copy_import_creates_file_meta_and_index_entry() {
    let h = Harness::new();
    let src = h.write_source("shot_010.psd", b"psd-binary-data");

    // Ensure dest parent exists
    std::fs::create_dir_all(h.project_dir.path().join("assets")).unwrap();

    let reqs = vec![ImportRequest {
        source: src.clone(),
        dest: p("assets/shot_010.psd"),
        mode: ImportMode::Copy,
        group_id: None,
    }];

    let preview = build_preview(&reqs, &h.fs, h.project_dir.path());
    assert!(preview.is_clean());

    let outcome = h.import_driver().apply(&preview).unwrap();
    assert_eq!(outcome.imported.len(), 1);

    // File landed in project
    assert!(h.project_file_exists("assets/shot_010.psd"));
    assert_eq!(
        h.project_file_content("assets/shot_010.psd"),
        b"psd-binary-data"
    );

    // Source still exists (copy mode)
    assert!(src.exists());

    // .meta sidecar was created
    assert!(h.project_file_exists("assets/shot_010.psd.meta"));

    // Index row was registered
    let row = h.index.get_file_by_path(&p("assets/shot_010.psd")).unwrap();
    assert!(row.is_some());

    // History entry was appended
    let entries = h.history.list(10).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].op.kind(), OpKind::Import);
}

#[test]
fn move_import_removes_source() {
    let h = Harness::new();
    let src = h.write_source("plate_v001.exr", b"exr-data");

    std::fs::create_dir_all(h.project_dir.path().join("plates")).unwrap();

    let reqs = vec![ImportRequest {
        source: src.clone(),
        dest: p("plates/plate_v001.exr"),
        mode: ImportMode::Move,
        group_id: None,
    }];

    let preview = build_preview(&reqs, &h.fs, h.project_dir.path());
    assert!(preview.is_clean());

    let outcome = h.import_driver().apply(&preview).unwrap();
    assert_eq!(outcome.imported.len(), 1);
    assert_eq!(outcome.imported[0].mode, ImportMode::Move);

    // File landed in project
    assert!(h.project_file_exists("plates/plate_v001.exr"));

    // Source was removed
    assert!(!src.exists());
}

#[test]
fn bulk_import_shares_group_id_in_history() {
    let h = Harness::new();
    let src1 = h.write_source("a.psd", b"a");
    let src2 = h.write_source("b.psd", b"b");
    let src3 = h.write_source("c.psd", b"c");

    std::fs::create_dir_all(h.project_dir.path().join("assets")).unwrap();

    let reqs = vec![
        ImportRequest {
            source: src1,
            dest: p("assets/a.psd"),
            mode: ImportMode::Copy,
            group_id: None,
        },
        ImportRequest {
            source: src2,
            dest: p("assets/b.psd"),
            mode: ImportMode::Copy,
            group_id: None,
        },
        ImportRequest {
            source: src3,
            dest: p("assets/c.psd"),
            mode: ImportMode::Copy,
            group_id: None,
        },
    ];

    let preview = build_preview(&reqs, &h.fs, h.project_dir.path());
    let outcome = h.import_driver().apply(&preview).unwrap();

    assert!(outcome.group_id.is_some());
    let group = outcome.group_id.unwrap();

    let entries = h.history.list(10).unwrap();
    assert_eq!(entries.len(), 3);
    for e in &entries {
        assert_eq!(e.group_id.as_deref(), Some(group.as_str()));
    }
}

#[test]
fn import_refuses_when_preview_has_conflicts() {
    let h = Harness::new();

    let reqs = vec![ImportRequest {
        source: "/nonexistent/file.psd".into(),
        dest: p("assets/file.psd"),
        mode: ImportMode::Copy,
        group_id: None,
    }];

    let preview = build_preview(&reqs, &h.fs, h.project_dir.path());
    assert!(!preview.is_clean());

    let result = h.import_driver().apply(&preview);
    assert!(result.is_err());
}

#[test]
fn caller_group_id_is_preserved() {
    let h = Harness::new();
    let src1 = h.write_source("frame_0001.exr", b"f1");
    let src2 = h.write_source("frame_0002.exr", b"f2");

    std::fs::create_dir_all(h.project_dir.path().join("seq")).unwrap();

    let reqs = vec![
        ImportRequest {
            source: src1,
            dest: p("seq/frame_0001.exr"),
            mode: ImportMode::Copy,
            group_id: Some("seq-abc123".into()),
        },
        ImportRequest {
            source: src2,
            dest: p("seq/frame_0002.exr"),
            mode: ImportMode::Copy,
            group_id: Some("seq-abc123".into()),
        },
    ];

    let preview = build_preview(&reqs, &h.fs, h.project_dir.path());
    let outcome = h.import_driver().apply(&preview).unwrap();

    assert_eq!(outcome.group_id.as_deref(), Some("seq-abc123"));
    let entries = h.history.list(10).unwrap();
    for e in &entries {
        assert_eq!(e.group_id.as_deref(), Some("seq-abc123"));
    }
}
