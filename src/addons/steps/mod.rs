use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

pub mod append;
pub mod copy;
pub mod create;
pub mod delete;
pub mod inject;
pub mod move_step;
pub mod rename;
pub mod replace;

pub enum Rollback {
  DeleteCreatedFile { path: PathBuf },
  RestoreFile { path: PathBuf, original: Vec<u8> },
  RenameFile { from: PathBuf, to: PathBuf },
}

pub fn render_lines(lines: &[String], ctx: &tera::Context) -> Result<Vec<String>> {
  lines
    .iter()
    .map(|line| tera::Tera::one_off(line, ctx, false).map_err(Into::into))
    .collect()
}

pub fn render_string(s: &str, ctx: &tera::Context) -> Result<String> {
  tera::Tera::one_off(s, ctx, false).map_err(Into::into)
}

fn normalize_join(root: &Path, relative: &str) -> PathBuf {
  let joined = root.join(relative);
  let mut out = PathBuf::new();
  for component in joined.components() {
    match component {
      Component::ParentDir => {
        out.pop();
      }
      Component::CurDir => {}
      c => out.push(c),
    }
  }
  out
}

fn deepest_existing_ancestor(path: &Path) -> PathBuf {
  let mut current = path;
  loop {
    if current.exists() {
      return current.to_path_buf();
    }
    match current.parent() {
      Some(parent) => current = parent,
      None => return current.to_path_buf(),
    }
  }
}

pub(super) fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  let canon_root = root
    .canonicalize()
    .with_context(|| format!("Cannot resolve project root '{}'", root.display()))?;

  let candidate = normalize_join(&canon_root, relative);

  if !candidate.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' would escape the root directory",
      label,
      relative
    ));
  }

  let canon_existing = deepest_existing_ancestor(&candidate)
    .canonicalize()
    .with_context(|| format!("Cannot resolve {} '{}'", label, relative))?;
  if !canon_existing.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' resolves outside the root directory via a symlink",
      label,
      relative
    ));
  }

  Ok(candidate)
}

pub(super) fn resolve_target(
  target: &crate::addons::manifest::Target,
  project_root: &Path,
) -> Result<Vec<PathBuf>> {
  use crate::addons::manifest::Target;
  match target {
    Target::File { file } => {
      let path = safe_join(project_root, file, "target file")?;
      Ok(vec![path])
    }
    Target::Glob { glob } => {
      safe_join(project_root, glob, "glob pattern")?;
      let pattern = project_root.join(glob).to_string_lossy().to_string();
      let canonical_root = project_root
        .canonicalize()
        .with_context(|| format!("Cannot resolve project root '{}'", project_root.display()))?;
      let paths = glob::glob(&pattern)?
        .filter_map(|e| e.ok())
        .filter(|p| {
          p.canonicalize()
            .map(|cp| cp.starts_with(&canonical_root))
            .unwrap_or(false)
        })
        .collect();
      Ok(paths)
    }
  }
}
