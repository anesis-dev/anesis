//! Template types and sub-modules.
//!
//! A "template" is a directory of project source files (some with `.tera`
//! extension for Tera-rendered content) alongside an `anesis.template.json`
//! manifest.  Sub-modules handle the full lifecycle:
//!
//! - [`install`]   — download from registry and cache locally
//! - [`loader`]    — resolve files from cache or linked directory
//! - [`generator`] — render `.tera` files and write project to disk
//! - [`publish`]   — register a GitHub repo on the backend registry
//! - [`update`]    — update an existing registry entry
//! - [`link`]      — register a local directory for development use

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod generator;
pub mod install;
pub mod loader;
pub mod publish;
pub mod update;

/// An in-memory representation of a single file within a template.
///
/// `path` is relative to the template root; `contents` are the raw bytes.
/// The generator module inspects the filename extension to decide whether
/// to render with Tera or copy as-is.
pub struct TemplateFile {
  pub path: PathBuf,
  pub contents: Vec<u8>,
}

/// Deserialised `anesis.template.json` manifest.
///
/// Every template repository must include this file at its root.
#[derive(Serialize, Deserialize)]
pub struct AnesisTemplate {
  /// Unique template name used as the cache key (e.g. `"react-vite-ts"`).
  pub name: String,
  /// Semantic version of the template (e.g. `"1.0.0"`).
  pub version: String,
  /// Minimum Anesis CLI version required to use this template.
  #[serde(rename = "anesisVersion")]
  pub anesis_version: String,
  pub repository: AnesisTemplateRepository,
  pub metadata: AnesisTemplateMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateRepository {
  /// GitHub URL of the template source repository.
  pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnesisTemplateMetadata {
  /// Human-readable name shown in the registry UI.
  #[serde(rename = "displayName")]
  pub display_name: String,
  pub description: String,
}
