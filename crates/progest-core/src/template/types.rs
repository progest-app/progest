use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fs::IgnoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDocument {
    pub meta: TemplateMeta,
    pub directories: Vec<String>,
    #[serde(default, skip_serializing_if = "IncludeSection::is_empty")]
    pub include: IncludeSection,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirmeta: Vec<DirmetaEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub progest_version: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncludeSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_toml: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_toml: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views_toml: Option<String>,
}

impl IncludeSection {
    pub fn is_empty(&self) -> bool {
        self.rules_toml.is_none() && self.schema_toml.is_none() && self.views_toml.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirmetaEntry {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ExportOptions {
    pub include_rules: bool,
    pub include_schema: bool,
    pub include_views: bool,
    pub include_dirmeta: bool,
}

impl ExportOptions {
    pub fn all() -> Self {
        Self {
            include_rules: true,
            include_schema: true,
            include_views: true,
            include_dirmeta: true,
        }
    }

    pub fn from_include_str(s: &str) -> Self {
        if s == "all" {
            return Self::all();
        }
        let mut opts = Self::default();
        for part in s.split(',') {
            match part.trim() {
                "rules" => opts.include_rules = true,
                "schema" => opts.include_schema = true,
                "views" => opts.include_views = true,
                "dirmeta" => opts.include_dirmeta = true,
                _ => {}
            }
        }
        opts
    }
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("template parse: {0}")]
    Parse(String),
    #[error("template I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("ignore rules: {0}")]
    Ignore(#[from] IgnoreError),
    #[error("template serialization: {0}")]
    Serialize(String),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyReport {
    pub directories_created: usize,
    pub configs_written: Vec<String>,
    pub dirmeta_written: usize,
}

pub fn serialize(doc: &TemplateDocument) -> Result<String, TemplateError> {
    toml::to_string_pretty(doc).map_err(|e| TemplateError::Serialize(e.to_string()))
}

pub fn deserialize(input: &str) -> Result<TemplateDocument, TemplateError> {
    toml::from_str(input).map_err(|e| TemplateError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_template() -> TemplateDocument {
        TemplateDocument {
            meta: TemplateMeta {
                id: "test-template".to_owned(),
                name: "Test Template".to_owned(),
                version: "1.0.0".to_owned(),
                author: String::new(),
                description: "A test template".to_owned(),
                progest_version: "0.1.0".to_owned(),
                created_at: "2026-04-30T00:00:00Z".to_owned(),
            },
            directories: vec!["assets/shots".to_owned(), "assets/scenes".to_owned()],
            include: IncludeSection {
                rules_toml: Some("schema_version = 1\n".to_owned()),
                schema_toml: None,
                views_toml: None,
            },
            dirmeta: vec![DirmetaEntry {
                path: "assets/shots".to_owned(),
                content: "schema_version = 1\n\n[accepts]\nexts = [\":image\"]\n".to_owned(),
            }],
        }
    }

    #[test]
    fn serialize_deserialize_round_trips() {
        let doc = sample_template();
        let toml_str = serialize(&doc).unwrap();
        let parsed = deserialize(&toml_str).unwrap();
        assert_eq!(parsed.meta.id, "test-template");
        assert_eq!(parsed.directories.len(), 2);
        assert!(parsed.include.rules_toml.is_some());
        assert!(parsed.include.schema_toml.is_none());
        assert_eq!(parsed.dirmeta.len(), 1);
        assert_eq!(parsed.dirmeta[0].path, "assets/shots");
    }

    #[test]
    fn empty_include_is_omitted() {
        let mut doc = sample_template();
        doc.include = IncludeSection::default();
        doc.dirmeta.clear();
        let toml_str = serialize(&doc).unwrap();
        assert!(!toml_str.contains("[include]"));
        assert!(!toml_str.contains("[[dirmeta]]"));
    }

    #[test]
    fn export_options_from_include_str() {
        let all = ExportOptions::from_include_str("all");
        assert!(
            all.include_rules && all.include_schema && all.include_views && all.include_dirmeta
        );

        let partial = ExportOptions::from_include_str("rules,dirmeta");
        assert!(partial.include_rules);
        assert!(!partial.include_schema);
        assert!(!partial.include_views);
        assert!(partial.include_dirmeta);
    }
}
