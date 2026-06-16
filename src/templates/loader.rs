//! Loads template files from the local cache, auto-installing if necessary.
//!
//! [`get_files`] is the single entry point used by the `new` command.  It
//! calls `install_template` to ensure the cache is fresh, then reads from the
//! cache directory.
//!
//! The separation between loader and generator allows tests to supply
//! `TemplateFile` slices directly without touching the filesystem.

use std::path::Path;

use anyhow::Result;
use log::debug;

use crate::{
  AppContext,
  templates::{TemplateFile, install::install_template},
  utils::fs::read_dir_to_files,
};

/// Returns all files in the template, auto-installing from the registry if needed.
pub async fn get_files(ctx: &AppContext, template_name: &str) -> Result<Vec<TemplateFile>> {
  let path = Path::new(template_name);
  debug!("Start install template");
  let install_result = install_template(ctx, template_name).await?;
  debug!("End install template");
  if let Some(message) = install_result.message(template_name) {
    println!("{message}");
  }

  debug!("Start read files");
  let files = read_dir_to_files(&ctx.paths.templates.join(path))?;
  debug!("End read files and return {} files", files.len());

  Ok(files)
}
