//! Resolved filesystem paths for every file and directory Anesis uses.
//!
//! All data is stored under `~/.anesis/`.  [`AnesisPaths`] is computed once at
//! startup and shared via [`AppContext`](crate::AppContext) so individual
//! modules never need to re-derive paths from the home directory.

use std::{fs, path::PathBuf};

use anyhow::Result;

/// Canonical paths to every file and directory owned by Anesis.
///
/// All fields are absolute `PathBuf` values derived from the OS home directory.
pub struct AnesisPaths {
  /// `~/.anesis/` — root of all Anesis data.
  pub home: PathBuf,
  /// `~/.anesis/config.json` — user configuration (currently unused).
  pub config: PathBuf,
  /// `~/.anesis/version_check.json` — cache of the last known latest version.
  pub version_check: PathBuf,
  /// `~/.anesis/cache/` — parent directory for all cached assets.
  pub cache: PathBuf,
  /// `~/.anesis/cache/templates/` — downloaded template directories.
  pub templates: PathBuf,
  /// `~/.anesis/auth.json` — stored JWT and username after `anesis login`.
  pub auth: PathBuf,
  /// `~/.anesis/cache/addons/` — downloaded addon directories.
  pub addons: PathBuf,
  /// `~/.anesis/cache/addons/anesis-addons.json` — addon cache index.
  pub addons_index: PathBuf,
}

impl AnesisPaths {
  /// Resolves all paths relative to the OS home directory.
  ///
  /// Returns an error if the home directory cannot be determined (unusual, but
  /// possible in minimal container environments).
  pub fn new() -> Result<Self> {
    let home_dir =
      dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let anesis_home = home_dir.join(".anesis");

    Ok(Self {
      home: anesis_home.clone(),
      config: anesis_home.join("config.json"),
      version_check: anesis_home.join("version_check.json"),
      cache: anesis_home.join("cache"),
      templates: anesis_home.join("cache").join("templates"),
      auth: anesis_home.join("auth.json"),
      addons: anesis_home.join("cache").join("addons"),
      addons_index: anesis_home
        .join("cache")
        .join("addons")
        .join("anesis-addons.json"),
    })
  }

  /// Creates all required directories if they don't already exist.
  ///
  /// Called once at startup before any command handler runs.
  /// `create_dir_all` is idempotent, so this is safe to call repeatedly.
  pub fn ensure_directories(&self) -> Result<()> {
    fs::create_dir_all(&self.home)?;
    fs::create_dir_all(&self.cache)?;
    fs::create_dir_all(&self.templates)?;
    fs::create_dir_all(&self.addons)?;
    Ok(())
  }
}
