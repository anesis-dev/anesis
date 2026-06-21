use std::{
  fs,
  path::{Component, Path, PathBuf},
};

use anyhow::{Result, anyhow};
use tera::{Context, Tera};

use crate::{
  cache::get_cached_template,
  context::AppContext,
  manifest::AnesisManifest,
  templates::{AnesisTemplate, TemplateFile},
};

pub fn extract_template(
  files: &[TemplateFile],
  project_name: &str,
  ctx: &AppContext,
) -> Result<()> {
  let output_path = PathBuf::from(project_name);
  fs::create_dir_all(&output_path)?;

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));

  let mut tera = Tera::default();

  extract_dir_contents(files, &output_path, &mut tera, &context, ctx)?;

  Ok(())
}

pub fn to_kebab_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '_' | ' ' => '-',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

pub fn to_snake_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '-' | ' ' => '_',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

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

pub fn to_camel_case(s: &str) -> String {
  let pascal = to_pascal_case(s);
  let mut chars = pascal.chars();
  match chars.next() {
    None => String::new(),
    Some(first) => first.to_lowercase().to_string() + chars.as_str(),
  }
}

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

pub fn extract_dir_contents(
  files: &[TemplateFile],
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
  ctx: &AppContext,
) -> Result<()> {
  for file in files {
    let file_name = file
      .path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", file.path.display()))?;
    let file_name_str = file_name.to_string_lossy();
    if file_name_str == "anesis.template.json" {
      let template: AnesisTemplate = serde_json::from_slice(&file.contents)?;
      let template_name = template.name;
      let cached_template = get_cached_template(ctx, &template_name)?.ok_or_else(|| {
        anyhow!("template '{template_name}' not found in cache; cannot write anesis.json")
      })?;
      AnesisManifest::new(&template_name, &cached_template.commit_sha, Vec::new())
        .write(base_path)?;
      continue;
    }
    let template_key = file.path.to_string_lossy();

    let output_path = safe_template_path(base_path, &file.path)?;
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    if let Some(output_name) = file_name_str.strip_suffix(".tera")
      && !output_name.is_empty()
    {
      let output_path = output_path.with_file_name(output_name);

      let template_content = std::str::from_utf8(&file.contents)?;
      tera.add_raw_template(&template_key, template_content)?;
      let rendered = tera.render(&template_key, context)?;

      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      fs::write(&output_path, &file.contents)?;
      println!("  ✓ {}", output_path.display());
    }
  }
  Ok(())
}
