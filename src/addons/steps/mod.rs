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

/// Renders content lines with Tera one_off — substitutes {{ var }} from user inputs.
pub fn render_lines(lines: &[String], ctx: &tera::Context) -> Result<Vec<String>> {
  lines
    .iter()
    .map(|line| tera::Tera::one_off(line, ctx, false).map_err(Into::into))
    .collect()
}

/// Renders a single string with Tera — used for dynamic file paths.
pub fn render_string(s: &str, ctx: &tera::Context) -> Result<String> {
  tera::Tera::one_off(s, ctx, false).map_err(Into::into)
}

/// Normalises `root.join(relative)` without touching the filesystem by
/// resolving `.` and `..` components lexically.
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

/// Returns the deepest ancestor of `path` (including `path` itself) that exists
/// on disk. Used to canonicalize the real portion of a path whose final
/// components may not have been created yet (e.g. `create` steps).
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

/// Joins `root` with `relative`, then verifies the result stays within `root`.
/// Returns the resolved path or an error if it would escape `root`.
///
/// This prevents path-traversal attacks in addon manifests (e.g. `../../etc/passwd`).
/// The root is `canonicalize`d first so the check operates on an absolute,
/// symlink-resolved path — this also closes the degenerate case where a relative
/// root normalised to an empty path, which made `starts_with("")` always true.
pub(super) fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  let canon_root = root
    .canonicalize()
    .with_context(|| format!("Cannot resolve project root '{}'", root.display()))?;

  // Lexically resolve `.`/`..` against the canonical (absolute) root.
  let candidate = normalize_join(&canon_root, relative);

  // Reject anything that escapes the root lexically (e.g. `../../etc/passwd`).
  if !candidate.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' would escape the root directory",
      label,
      relative
    ));
  }

  // Defend against symlinks: canonicalize the deepest existing portion of the
  // target and confirm it still lives inside the canonical root. A symlink under
  // the root that points outside it would otherwise let writes escape.
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
      // Validate the pattern itself doesn't traverse outside root
      safe_join(project_root, glob, "glob pattern")?;
      let pattern = project_root.join(glob).to_string_lossy().to_string();
      let canonical_root = project_root
        .canonicalize()
        .with_context(|| format!("Cannot resolve project root '{}'", project_root.display()))?;
      let paths = glob::glob(&pattern)?
        .filter_map(|e| e.ok())
        // Filter out any results that escape root via symlinks
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
