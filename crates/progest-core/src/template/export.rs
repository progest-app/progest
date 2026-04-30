use std::fs;
use std::path::Path;

use crate::fs::{DEFAULT_PATTERNS, EntryKind, IgnoreRules, Scanner};
use crate::project::ProjectRoot;

use super::types::{
    DirmetaEntry, ExportOptions, IncludeSection, TemplateDocument, TemplateError, TemplateMeta,
};

pub fn export_template(
    root: &ProjectRoot,
    name: &str,
    options: &ExportOptions,
) -> Result<TemplateDocument, TemplateError> {
    let directories = collect_directories(root)?;
    let include = collect_includes(root, options)?;
    let dirmeta = if options.include_dirmeta {
        collect_dirmeta(root, &directories)?
    } else {
        Vec::new()
    };

    let meta = TemplateMeta {
        id: uuid::Uuid::now_v7().simple().to_string(),
        name: name.to_owned(),
        version: "1.0.0".to_owned(),
        author: String::new(),
        description: String::new(),
        progest_version: crate::VERSION.to_owned(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(TemplateDocument {
        meta,
        directories,
        include,
        dirmeta,
    })
}

fn collect_directories(root: &ProjectRoot) -> Result<Vec<String>, TemplateError> {
    let rules = IgnoreRules::from_patterns(root.root(), DEFAULT_PATTERNS.iter().copied())?;
    let scanner = Scanner::new(root.root().to_path_buf(), rules);

    let mut dirs: Vec<String> = scanner
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.kind != EntryKind::Dir {
                return None;
            }
            let path_str = entry.path.as_str().to_owned();
            if path_str.is_empty() {
                return None;
            }
            Some(path_str)
        })
        .collect();

    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

fn collect_includes(
    root: &ProjectRoot,
    options: &ExportOptions,
) -> Result<IncludeSection, TemplateError> {
    Ok(IncludeSection {
        rules_toml: if options.include_rules {
            read_optional_file(&root.rules_toml())?
        } else {
            None
        },
        schema_toml: if options.include_schema {
            read_optional_file(&root.schema_toml())?
        } else {
            None
        },
        views_toml: if options.include_views {
            read_optional_file(&root.views_toml())?
        } else {
            None
        },
    })
}

fn collect_dirmeta(
    root: &ProjectRoot,
    directories: &[String],
) -> Result<Vec<DirmetaEntry>, TemplateError> {
    let mut entries = Vec::new();

    let root_dirmeta = root.root().join(".dirmeta.toml");
    if let Some(content) = read_optional_file(&root_dirmeta)? {
        entries.push(DirmetaEntry {
            path: ".".to_owned(),
            content,
        });
    }

    for dir in directories {
        let dirmeta_path = root.root().join(dir).join(".dirmeta.toml");
        if let Some(content) = read_optional_file(&dirmeta_path)? {
            entries.push(DirmetaEntry {
                path: dir.clone(),
                content,
            });
        }
    }

    Ok(entries)
}

fn read_optional_file(path: &Path) -> Result<Option<String>, TemplateError> {
    if path.is_file() {
        Ok(Some(fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::initialize;
    use tempfile::TempDir;

    #[test]
    fn export_captures_directory_structure() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("demo");
        let root = initialize(&project, "Demo").unwrap();

        fs::create_dir_all(root.root().join("assets/shots")).unwrap();
        fs::create_dir_all(root.root().join("assets/scenes")).unwrap();

        let doc = export_template(&root, "Demo Template", &ExportOptions::default()).unwrap();
        assert!(doc.directories.contains(&"assets".to_owned()));
        assert!(doc.directories.contains(&"assets/shots".to_owned()));
        assert!(doc.directories.contains(&"assets/scenes".to_owned()));
    }

    #[test]
    fn export_includes_rules_when_present() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("demo");
        let root = initialize(&project, "Demo").unwrap();

        let rules_content = "schema_version = 1\n";
        fs::write(root.rules_toml(), rules_content).unwrap();

        let doc = export_template(&root, "Demo", &ExportOptions::all()).unwrap();
        assert_eq!(doc.include.rules_toml.as_deref(), Some(rules_content));
    }

    #[test]
    fn export_skips_configs_when_not_requested() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("demo");
        let root = initialize(&project, "Demo").unwrap();

        fs::write(root.rules_toml(), "schema_version = 1\n").unwrap();

        let doc = export_template(&root, "Demo", &ExportOptions::default()).unwrap();
        assert!(doc.include.rules_toml.is_none());
    }

    #[test]
    fn export_collects_dirmeta_files() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("demo");
        let root = initialize(&project, "Demo").unwrap();

        let shots_dir = root.root().join("assets/shots");
        fs::create_dir_all(&shots_dir).unwrap();
        fs::write(
            shots_dir.join(".dirmeta.toml"),
            "schema_version = 1\n\n[accepts]\nexts = [\":image\"]\n",
        )
        .unwrap();

        let opts = ExportOptions {
            include_dirmeta: true,
            ..ExportOptions::default()
        };
        let doc = export_template(&root, "Demo", &opts).unwrap();
        assert_eq!(doc.dirmeta.len(), 1);
        assert_eq!(doc.dirmeta[0].path, "assets/shots");
        assert!(doc.dirmeta[0].content.contains("[accepts]"));
    }
}
