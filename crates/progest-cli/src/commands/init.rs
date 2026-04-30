//! `progest init` — bootstrap a `.progest/` layout in the current directory.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use progest_core::project;
use progest_core::template;

pub fn run(cwd: &Path, name: Option<String>, template_path: Option<PathBuf>) -> Result<()> {
    let default_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();
    let name = name.unwrap_or(default_name);

    let root = project::initialize(cwd, &name)
        .with_context(|| format!("failed to initialize project at `{}`", cwd.display()))?;

    println!(
        "Initialized Progest project `{name}` at {}",
        root.root().display()
    );
    println!("  • {}", root.project_toml().display());
    println!("  • {}", root.user_ignore().display());
    println!("  • {}", root.index_db().display());

    if let Some(tpl_path) = template_path {
        let tpl_content = std::fs::read_to_string(&tpl_path)
            .with_context(|| format!("failed to read template `{}`", tpl_path.display()))?;
        let doc = template::deserialize(&tpl_content)
            .with_context(|| format!("failed to parse template `{}`", tpl_path.display()))?;
        let report = template::apply_template(root.root(), &doc)
            .with_context(|| "failed to apply template")?;

        println!("\nTemplate `{}` applied:", doc.meta.name);
        println!("  {} directories created", report.directories_created);
        if !report.configs_written.is_empty() {
            println!("  configs: {}", report.configs_written.join(", "));
        }
        if report.dirmeta_written > 0 {
            println!("  {} .dirmeta.toml written", report.dirmeta_written);
        }
    }

    println!("\nNext steps: drop some files in the project and run `progest scan`.");
    Ok(())
}
