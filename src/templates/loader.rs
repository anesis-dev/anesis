use std::path::Path;

use anyhow::Result;
use log::debug;

use crate::{
  context::AppContext,
  templates::{TemplateFile, install::install_template},
  utils::fs::read_dir_to_files,
};

pub async fn get_files(ctx: &AppContext, template_name: &str) -> Result<Vec<TemplateFile>> {
  debug!("Start install template");
  let install_result = install_template(ctx, template_name).await?;
  debug!("End install template");
  if let Some(message) = install_result.message(template_name) {
    println!("{message}");
  }

  let path = Path::new(template_name);
  debug!("Start read files");
  let files = read_dir_to_files(&ctx.paths.templates.join(path))?;
  debug!("End read files and return {} files", files.len());

  Ok(files)
}
