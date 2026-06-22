use std::{fs, path::PathBuf};

use anyhow::Result;

pub struct AnesisPaths {
  pub home: PathBuf,
  pub config: PathBuf,
  pub version_check: PathBuf,
  pub cache: PathBuf,
  pub templates: PathBuf,
  pub auth: PathBuf,
  pub addons: PathBuf,
  pub addons_index: PathBuf,
}

impl AnesisPaths {
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

  pub fn ensure_directories(&self) -> Result<()> {
    fs::create_dir_all(&self.home)?;
    fs::create_dir_all(&self.cache)?;
    fs::create_dir_all(&self.templates)?;
    fs::create_dir_all(&self.addons)?;
    Ok(())
  }
}
