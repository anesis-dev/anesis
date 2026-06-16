//! Tera-based template rendering and project scaffolding.
//!
//! [`extract_template`] is the main entry point.  It iterates over
//! `TemplateFile` slices and either:
//! - Renders files ending in `.tera` through the Tera engine (stripping the
//!   `.tera` suffix from the output filename).
//! - Copies all other files verbatim.
//!
//! Path traversal within archives is guarded by [`safe_template_path`], which
//! lexically normalises paths and verifies the result stays inside the output
//! directory.

use std::{
  fs,
  path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow};
use tera::{Context, Tera};

use crate::templates::TemplateFile;

/// Renders a template into the current working directory under `project_name/`.
///
/// Available Tera context variables:
/// - `project_name`       — exactly as typed by the user
/// - `project_name_kebab` — lowercase kebab-case
/// - `project_name_snake` — lowercase snake_case
pub fn extract_template(files: &[TemplateFile], project_name: &str) -> Result<()> {
  let output_path = PathBuf::from(project_name);
  fs::create_dir_all(&output_path)?;

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));

  // A single Tera instance is reused across all files so templates can
  // theoretically include each other (though no templates use that currently).
  let mut tera = Tera::default();

  extract_dir_contents(files, &output_path, &mut tera, &context)?;

  Ok(())
}

/// Converts `_` and spaces to `-` and lowercases the result.
pub fn to_kebab_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '_' | ' ' => '-',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

/// Converts `-` and spaces to `_` and lowercases the result.
pub fn to_snake_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '-' | ' ' => '_',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

/// Capitalises each word (split on `_`, `-`, or space) and joins without separator.
pub fn to_pascal_case(s: &str) -> String {
  s.split(['_', '-', ' '])
    .filter(|p| !p.is_empty())
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
      }
    })
    .collect()
}

/// Converts to PascalCase then lowercases the first character.
pub fn to_camel_case(s: &str) -> String {
  let pascal = to_pascal_case(s);
  let mut chars = pascal.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_lowercase().to_string() + chars.as_str(),
  }
}

/// Normalises `base.join(relative)` lexically (no filesystem I/O) and
/// verifies the result stays within `base`.  Prevents path-traversal in
/// template archives (e.g. `../../.bashrc`).
fn safe_template_path(base: &Path, relative: &Path) -> Result<PathBuf> {
  let joined = base.join(relative);
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
  // Normalise base the same way for comparison
  let mut norm_base = PathBuf::new();
  for component in base.components() {
    match component {
      Component::ParentDir => {
        norm_base.pop();
      }
      Component::CurDir => {}
      c => norm_base.push(c),
    }
  }
  if !out.starts_with(&norm_base) {
    return Err(anyhow!(
      "Path traversal blocked: template file '{}' would escape the output directory",
      relative.display()
    ));
  }
  Ok(out)
}

/// Renders or copies all `files` under `base_path`.
///
/// `.tera` files are registered with the Tera engine under their relative
/// path as the template key.  Using the relative path as the key ensures
/// that two files with the same name in different subdirectories don't
/// collide.
pub fn extract_dir_contents(
  files: &[TemplateFile],
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
) -> Result<()> {
  for file in files {
    let file_name = file
      .path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", file.path.display()))?;
    let file_name_str = file_name.to_string_lossy();
    let template_key = file.path.to_string_lossy();

    let output_path = safe_template_path(base_path, &file.path)?;
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    if let Some(output_name) = file_name_str.strip_suffix(".tera")
      && !output_name.is_empty()
    {
      // Render the Tera template and strip the `.tera` suffix from the output file.
      let output_path = output_path.with_file_name(output_name);

      let template_content = std::str::from_utf8(&file.contents)?;
      tera.add_raw_template(&template_key, template_content)?;
      let rendered = tera.render(&template_key, context)?;

      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      // Non-template file: copy as-is (images, binaries, plain text, etc.).
      fs::write(&output_path, &file.contents)?;
      println!("  ✓ {}", output_path.display());
    }
  }
  Ok(())
}
