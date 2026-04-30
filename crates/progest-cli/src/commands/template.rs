use std::path::Path;

use anyhow::{Context, Result};

use progest_core::project::ProjectDocument;
use progest_core::template::{self, ApplyReport, ExportOptions};

use crate::context;
use crate::output::{self, OutputFormat};

pub struct TemplateExportArgs {
    pub out: std::path::PathBuf,
    pub include: String,
    pub name: Option<String>,
    pub format: OutputFormat,
}

pub fn run_export(cwd: &Path, args: &TemplateExportArgs) -> Result<i32> {
    let root = context::discover_root(cwd)?;

    let project_name = args.name.clone().unwrap_or_else(|| {
        let toml_path = root.project_toml();
        std::fs::read_to_string(&toml_path)
            .ok()
            .and_then(|s| ProjectDocument::from_toml_str(&s).ok())
            .map_or_else(
                || {
                    root.root()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("project")
                        .to_owned()
                },
                |doc| doc.name,
            )
    });

    let options = ExportOptions::from_include_str(&args.include);
    let doc = template::export_template(&root, &project_name, &options)
        .context("failed to export template")?;

    let toml_str = template::serialize(&doc).context("failed to serialize template")?;
    std::fs::write(&args.out, &toml_str)
        .with_context(|| format!("failed to write template to `{}`", args.out.display()))?;

    match args.format {
        OutputFormat::Json => {
            output::emit_json(&doc, "template export")?;
        }
        OutputFormat::Text => {
            println!(
                "Exported template `{}` to {}",
                doc.meta.name,
                args.out.display()
            );
            println!("  {} directories", doc.directories.len());
            if doc.include.rules_toml.is_some() {
                println!("  + rules.toml");
            }
            if doc.include.schema_toml.is_some() {
                println!("  + schema.toml");
            }
            if doc.include.views_toml.is_some() {
                println!("  + views.toml");
            }
            if !doc.dirmeta.is_empty() {
                println!("  + {} .dirmeta.toml", doc.dirmeta.len());
            }
        }
    }
    Ok(0)
}

pub struct TemplateApplyArgs {
    pub template: std::path::PathBuf,
    pub format: OutputFormat,
}

pub fn run_apply(cwd: &Path, args: &TemplateApplyArgs) -> Result<i32> {
    let root = context::discover_root(cwd)?;

    let content = std::fs::read_to_string(&args.template)
        .with_context(|| format!("failed to read template `{}`", args.template.display()))?;
    let doc = template::deserialize(&content)
        .with_context(|| format!("failed to parse template `{}`", args.template.display()))?;
    let report = template::apply_template(root.root(), &doc).context("failed to apply template")?;

    emit_apply_report(&doc.meta.name, &report, args.format)?;
    Ok(0)
}

fn emit_apply_report(name: &str, report: &ApplyReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => output::emit_json(report, "template apply"),
        OutputFormat::Text => {
            println!("Applied template `{name}`:");
            println!("  {} directories created", report.directories_created);
            if !report.configs_written.is_empty() {
                println!("  configs: {}", report.configs_written.join(", "));
            }
            if report.dirmeta_written > 0 {
                println!("  {} .dirmeta.toml written", report.dirmeta_written);
            }
            Ok(())
        }
    }
}
