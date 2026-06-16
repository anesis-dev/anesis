use std::{fs, path::Path};

use anyhow::Result;
use chrono::Utc;
use comfy_table::{Attribute, Cell, Table};
use serde::{Deserialize, Serialize};

use super::manifest::AddonManifest;

#[derive(Serialize, Deserialize, Debug)]
pub struct AddonsCache {
  #[serde(rename = "lastUpdated")]
  pub last_updated: String,
  pub addons: Vec<CachedAddon>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CachedAddon {
  pub id: String,
  pub name: String,
  pub version: String,
  pub path: String,
  pub commit_sha: String,
  pub repo_url: String,
}

fn read_cache(addons_dir: &Path) -> Result<AddonsCache> {
  let index = addons_dir.join("anesis-addons.json");
  if index.exists() {
    let content = fs::read_to_string(&index)?;
    Ok(serde_json::from_str(&content)?)
  } else {
    Ok(AddonsCache {
      last_updated: Utc::now().to_rfc3339(),
      addons: Vec::new(),
    })
  }
}

fn write_cache(addons_dir: &Path, cache: &AddonsCache) -> Result<()> {
  let index = addons_dir.join("anesis-addons.json");
  fs::write(index, serde_json::to_string_pretty(cache)?)?;
  Ok(())
}

pub fn update_addons_cache(
  addons_dir: &Path,
  subdir: &str,
  manifest: &AddonManifest,
  commit_sha: &str,
) -> Result<()> {
  let mut cache = read_cache(addons_dir)?;

  cache.last_updated = Utc::now().to_rfc3339();
  cache.addons.retain(|a| a.id != manifest.id);
  cache.addons.push(CachedAddon {
    id: manifest.id.clone(),
    name: manifest.name.clone(),
    version: manifest.version.clone(),
    path: subdir.to_string(),
    commit_sha: commit_sha.to_string(),
    repo_url: String::new(),
  });

  write_cache(addons_dir, &cache)
}

pub fn get_cached_addon(addons_dir: &Path, addon_id: &str) -> Result<Option<CachedAddon>> {
  let cache = read_cache(addons_dir)?;
  Ok(cache.addons.into_iter().find(|a| a.id == addon_id))
}

pub fn remove_addon_from_cache(addons_dir: &Path, addon_id: &str) -> Result<()> {
  let mut cache = read_cache(addons_dir)?;

  let entry = cache
    .addons
    .iter()
    .find(|a| a.id == addon_id)
    .ok_or_else(|| anyhow::anyhow!("Addon '{}' is not installed", addon_id))?;

  let addon_dir = addons_dir.join(&entry.path);
  if addon_dir.exists() {
    fs::remove_dir_all(&addon_dir)?;
  }

  cache.last_updated = Utc::now().to_rfc3339();
  cache.addons.retain(|a| a.id != addon_id);

  write_cache(addons_dir, &cache)?;
  println!("✓ Removed addon '{}'", addon_id);
  Ok(())
}

pub fn get_installed_addons(addons_dir: &Path) -> Result<()> {
  let cache = read_cache(addons_dir)?;

  if cache.addons.is_empty() {
    println!("No addons installed yet.");
    return Ok(());
  }

  let mut table = Table::new();
  table.set_header(vec![
    Cell::new("ID").add_attribute(Attribute::Bold),
    Cell::new("Name").add_attribute(Attribute::Bold),
    Cell::new("Version").add_attribute(Attribute::Bold),
    Cell::new("Source").add_attribute(Attribute::Bold),
  ]);

  for addon in &cache.addons {
    table.add_row(vec![
      Cell::new(&addon.id),
      Cell::new(&addon.name),
      Cell::new(&addon.version),
      Cell::new("registry"),
    ]);
  }

  println!("\nInstalled addons (last updated: {}):", cache.last_updated);
  println!("{table}");

  Ok(())
}

pub fn is_addon_installed(addons_dir: &Path, addon_id: &str) -> Result<bool> {
  let cache = read_cache(addons_dir)?;
  let found = cache.addons.iter().find(|a| a.id == addon_id);
  match found {
    Some(entry) => Ok(addons_dir.join(&entry.path).exists()),
    None => Ok(false),
  }
}
