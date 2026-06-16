//! Template cache index management.
//!
//! The cache index lives at `~/.anesis/cache/templates/anesis-templates.json`
//! and tracks every template that has been installed locally.  Each entry
//! records the template name, version, and commit SHA (used for up-to-date
//! checks).
//!
//! The module also owns the "list" and "remove" operations that are exposed
//! through `anesis template list` and `anesis template remove`.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::Utc;
use comfy_table::{Attribute, Cell, Table};
use serde::{Deserialize, Serialize};

use crate::{AppContext, templates::AnesisTemplate};

/// Root structure of `anesis-templates.json`.
#[derive(Serialize, Deserialize)]
pub struct TemplatesCache {
  /// RFC 3339 timestamp of the last write; used for display only.
  #[serde(rename = "lastUpdated")]
  pub last_updated: String,
  pub templates: Vec<CachedTemplate>,
}

/// A single entry in the templates cache index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedTemplate {
  /// Template name as declared in `anesis.template.json`.
  pub name: String,
  pub version: String,
  /// Source repository URL.
  pub source: String,
  /// Path of the template directory *relative to* the templates cache root.
  pub path: String,
  /// The GitHub commit SHA at the time of download.
  pub commit_sha: String,
}

/// Reads `anesis.template.json` from the freshly-extracted template directory,
/// then upserts the entry in `anesis-templates.json`.
///
/// `template_path` — root of the templates cache (`~/.anesis/cache/templates/`).
/// `path`          — name of the template subdirectory (e.g. `"react-vite-ts"`).
/// `commit_sha`    — the GitHub commit SHA of the downloaded revision.
///
/// Returns the newly written [`CachedTemplate`] entry.
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

  // Replace existing entry to avoid duplicates on re-download.
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

/// Looks up a single template entry in the cache index by name.
///
/// Returns `Ok(None)` when the index file doesn't exist or the name is absent.
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

/// Removes a template from the cache index and deletes its cached files.
/// Walks up the directory tree removing empty parent directories so the cache
/// stays tidy after partial extractions.
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
      // Remove intermediate empty directories up to (but not including) the
      // templates cache root.  Stops at the first non-empty directory.
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

/// Prints a formatted table of all locally installed templates to stdout.
/// Used by `anesis template list`.
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

/// Returns `true` if the template is recorded in the cache index **and** its
/// directory exists on disk (both conditions must hold to avoid stale entries
/// reporting as installed after a manual cache wipe).
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
