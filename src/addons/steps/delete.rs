use std::path::Path;

use anyhow::Result;

use crate::addons::manifest::DeleteStep;

use super::{Rollback, resolve_target};

pub fn execute_delete(step: &DeleteStep, project_root: &Path) -> Result<Vec<Rollback>> {
  let paths = resolve_target(&step.target, project_root)?;
  let mut rollbacks = Vec::new();

  for path in paths {
    if !path.is_file() {
      continue;
    }
    let original = std::fs::read(&path)?;
    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
    });
    std::fs::remove_file(&path)?;
  }

  Ok(rollbacks)
}
