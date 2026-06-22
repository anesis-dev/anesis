use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod generator;
pub mod install;
pub mod loader;
pub mod publish;
pub mod update;

pub struct TemplateFile {
  pub path: PathBuf,
  pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplate {
  pub name: String,
  pub version: String,
  #[serde(rename = "anesisVersion")]
  pub anesis_version: String,
  pub repository: AnesisTemplateRepository,
  pub metadata: AnesisTemplateMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateRepository {
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateMetadata {
  #[serde(rename = "displayName")]
  pub display_name: String,
  pub description: String,
}
