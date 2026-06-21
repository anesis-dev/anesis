use std::{fs, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnesisManifest {
  pub template_name: String,
  pub template_sha: String,
  pub addons: Vec<String>,
}

impl AnesisManifest {
  pub fn new(template_name: &str, template_sha: &str, addons: Vec<String>) -> Self {
    Self {
      template_name: template_name.to_string(),
      template_sha: template_sha.to_string(),
      addons,
    }
  }

  pub fn write(self, path: &Path) -> Result<()> {
    let output_path = path.join("anesis.json");

    let bytes = serde_json::to_vec_pretty(&self)?;
    fs::write(&output_path, bytes)?;
    println!("  ✓ {}", output_path.display());
    Ok(())
  }

  pub fn add_addon(addon_name: &str, project_root: &Path) -> Result<()> {
    let path = project_root.join("anesis.json");
    if !path.exists() {
      return Ok(());
    }

    let contents = fs::read_to_string(&path)?;
    let mut anesis_json: AnesisManifest = serde_json::from_str(&contents)?;

    if anesis_json
      .addons
      .iter()
      .find(|a| *a == addon_name)
      .is_none()
    {
      anesis_json.addons.push(addon_name.to_string());
      let bytes = serde_json::to_vec_pretty(&anesis_json)?;

      fs::write(&path, bytes)?;
    }
    Ok(())
  }
}
