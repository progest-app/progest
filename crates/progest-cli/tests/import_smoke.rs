//! Smoke tests for `progest import`.

mod support;

use support::{init_project, run, write_file};
use tempfile::TempDir;

#[test]
fn import_copy_creates_file_in_project() {
    let project = TempDir::new().unwrap();
    init_project(project.path(), "smoke-import").unwrap();

    // Create a source file outside the project
    let source = TempDir::new().unwrap();
    write_file(source.path(), "shot_010.psd", "psd-data").unwrap();

    // Create dest dir
    std::fs::create_dir_all(project.path().join("assets")).unwrap();

    let src_path = source.path().join("shot_010.psd");
    let output = run(
        project.path(),
        &[
            "import",
            src_path.to_str().unwrap(),
            "--dest",
            "assets",
            "--format",
            "json",
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File was copied into project
    assert!(project.path().join("assets/shot_010.psd").exists());
    // Source still exists (copy mode)
    assert!(src_path.exists());

    // JSON output includes imported array
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["imported"].as_array().unwrap().len(), 1);
}

#[test]
fn import_move_removes_source() {
    let project = TempDir::new().unwrap();
    init_project(project.path(), "smoke-import-move").unwrap();

    let source = TempDir::new().unwrap();
    write_file(source.path(), "plate.exr", "exr-data").unwrap();

    let src_path = source.path().join("plate.exr");
    let output = run(
        project.path(),
        &["import", src_path.to_str().unwrap(), "--move"],
    );

    assert!(
        output.status.success(),
        "import --move failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // File in project
    assert!(project.path().join("plate.exr").exists());
    // Source removed
    assert!(!src_path.exists());
}

#[test]
fn import_dry_run_does_not_modify_filesystem() {
    let project = TempDir::new().unwrap();
    init_project(project.path(), "smoke-import-dry").unwrap();

    let source = TempDir::new().unwrap();
    write_file(source.path(), "preview.psd", "data").unwrap();

    let src_path = source.path().join("preview.psd");
    let output = run(
        project.path(),
        &["import", src_path.to_str().unwrap(), "--dry-run"],
    );

    assert!(output.status.success());
    // File NOT copied
    assert!(!project.path().join("preview.psd").exists());
    // Source still there
    assert!(src_path.exists());
}

#[test]
fn import_missing_file_exits_nonzero() {
    let project = TempDir::new().unwrap();
    init_project(project.path(), "smoke-import-missing").unwrap();

    let output = run(
        project.path(),
        &["import", "/nonexistent/file.psd", "--dry-run"],
    );

    assert_eq!(output.status.code(), Some(1));
}
