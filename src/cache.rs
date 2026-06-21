use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::Utc;
use comfy_table::{Attribute, Cell, Table};
use serde::{Deserialize, Serialize};

use crate::{context::AppContext, templates::AnesisTemplate};

#[derive(Serialize, Deserialize)]
pub struct TemplatesCache {
  #[serde(rename = "lastUpdated")]
  pub last_updated: String,
  pub templates: Vec<CachedTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedTemplate {
  pub name: String,
  pub version: String,
  pub source: String,
  pub path: String,
  pub commit_sha: String,
}

pub fn update_templates_cache(
  template_path: &Path,
  path: &Path,
  commit_sha: &str,
) -> Result<CachedTemplate> {
  let anesis_json = template_path.join(path).join("anesis.template.json");
  let content = fs::read_to_string(&anesis_json)
    .with_context(|| format!("Failed to read {}", anesis_json.display()))?;
  let template_info: AnesisTemplate = serde_json::from_str(&content)?;

  let templates_json = template_path.join("anesis-templates.json");
  let mut templates_info: TemplatesCache = if templates_json.exists() {
    let content = fs::read_to_string(&templates_json)?;
    serde_json::from_str(&content)?
  } else {
    TemplatesCache {
      last_updated: Utc::now().to_rfc3339(),
      templates: Vec::new(),
    }
  };

  templates_info.last_updated = Utc::now().to_rfc3339();

  templates_info
    .templates
    .retain(|t| t.name != template_info.name);
  let cached_template = CachedTemplate {
    name: template_info.name,
    version: template_info.version,
    source: template_info.repository.url,
    path: path.to_string_lossy().to_string(),
    commit_sha: commit_sha.to_string(),
  };
  templates_info.templates.push(cached_template.clone());

  fs::write(
    &templates_json,
    serde_json::to_string_pretty(&templates_info)?,
  )?;

  Ok(cached_template)
}

pub fn get_cached_template(ctx: &AppContext, name: &str) -> Result<Option<CachedTemplate>> {
  let templates_json = ctx.paths.templates.join("anesis-templates.json");

  if !templates_json.exists() {
    return Ok(None);
  }

  let content = fs::read_to_string(&templates_json)?;
  let templates_info: TemplatesCache = serde_json::from_str(&content)?;

  Ok(
    templates_info
      .templates
      .into_iter()
      .find(|t| t.name == name),
  )
}

pub fn remove_template_from_cache(template_path: &Path, template_name: &str) -> Result<()> {
  let templates_json = template_path.join("anesis-templates.json");

  if !templates_json.exists() {
    return Err(anyhow::anyhow!(
      "Template '{}' is not installed",
      template_name
    ));
  }

  let content = fs::read_to_string(&templates_json)?;
  let mut templates_info: TemplatesCache = serde_json::from_str(&content)?;

  let exists = templates_info
    .templates
    .iter()
    .any(|t| t.name == template_name);
  if !exists {
    return Err(anyhow::anyhow!(
      "Template '{}' is not installed",
      template_name
    ));
  }

  templates_info.last_updated = Utc::now().to_rfc3339();

  if let Some(t) = templates_info
    .templates
    .iter()
    .find(|t| t.name == template_name)
  {
    let cleanup_path = template_path.join(&t.path);
    if cleanup_path.exists() {
      if let Err(e) = fs::remove_dir_all(&cleanup_path) {
        eprintln!("Failed to remove: {}", e);
      }
      let mut current = cleanup_path.parent();
      while let Some(parent) = current {
        if parent == template_path {
          break;
        }
        if fs::remove_dir(parent).is_err() {
          break;
        }
        current = parent.parent();
      }
    }
  }

  templates_info
    .templates
    .retain(|template| template.name != template_name);

  fs::write(
    &templates_json,
    serde_json::to_string_pretty(&templates_info)?,
  )?;

  println!("✓ Removed template '{}'", template_name);
  Ok(())
}

pub fn get_installed_templates(template_path: &Path) -> Result<()> {
  let templates_json = template_path.join("anesis-templates.json");

  let templates_info: TemplatesCache = if templates_json.exists() {
    let content = fs::read_to_string(&templates_json)?;
    serde_json::from_str(&content)?
  } else {
    TemplatesCache {
      last_updated: Utc::now().to_rfc3339(),
      templates: Vec::new(),
    }
  };

  if templates_info.templates.is_empty() {
    println!("No templates installed yet.");
    return Ok(());
  }

  let mut table = Table::new();

  table.set_header(vec![
    Cell::new("Name").add_attribute(Attribute::Bold),
    Cell::new("Version").add_attribute(Attribute::Bold),
    Cell::new("Source").add_attribute(Attribute::Bold),
  ]);

  for template in templates_info.templates {
    table.add_row(vec![
      Cell::new(&template.name),
      Cell::new(&template.version),
      Cell::new("registry"),
    ]);
  }

  println!(
    "\nInstalled templates (last updated: {}):",
    templates_info.last_updated
  );
  println!("{table}");

  Ok(())
}

pub fn is_template_installed(ctx: &AppContext, template_name: &str) -> Result<bool> {
  let templates_json = ctx.paths.templates.join("anesis-templates.json");

  let templates_info: TemplatesCache = if templates_json.exists() {
    let content = fs::read_to_string(&templates_json)?;
    serde_json::from_str(&content)?
  } else {
    TemplatesCache {
      last_updated: Utc::now().to_rfc3339(),
      templates: Vec::new(),
    }
  };

  let path = Path::new(template_name);
  if !ctx.paths.templates.join(path).exists() {
    return Ok(false);
  }

  Ok(
    templates_info
      .templates
      .iter()
      .any(|t| t.name == template_name),
  )
}
