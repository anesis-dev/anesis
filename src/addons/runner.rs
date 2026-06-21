use std::{collections::HashMap, fs, path::Path};

use anyhow::{Result, anyhow};
use inquire::{Confirm, Select, Text};
use reqwest::StatusCode;

use crate::{
  context::AppContext,
  manifest::AnesisManifest,
  templates::generator::{to_camel_case, to_kebab_case, to_pascal_case, to_snake_case},
  utils::{errors::AnesisError, ui::spinner},
};

use super::{
  cache::is_addon_installed,
  detect::detect_variant,
  install::{AddonInstallResult, install_addon, read_cached_manifest, record_addon_use},
  lock::{LockEntry, LockFile},
  manifest::{InputDef, InputType},
  steps::{
    Rollback, append::execute_append, copy::execute_copy, create::execute_create,
    delete::execute_delete, inject::execute_inject, move_step::execute_move,
    rename::execute_rename, replace::execute_replace,
  },
};
use crate::addons::manifest::Step;

pub async fn run_addon_command(
  ctx: &AppContext,
  addon_id: &str,
  command_name: &str,
  project_root: &Path,
) -> Result<()> {
  let addon_is_cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?.is_some();
  let sp = spinner(format!("Checking addon '{addon_id}' for updates..."));
  let install_result = match install_addon(ctx, addon_id).await {
    Ok(install_result) => {
      sp.finish_and_clear();
      install_result
    }
    Err(err) if addon_is_cached && should_fallback_to_cached_manifest(&err) => {
      sp.finish_and_clear();
      eprintln!(
        "Note: Could not check for addon updates ({}). Using cached version.",
        err
      );
      AddonInstallResult::UpToDate(read_cached_manifest(&ctx.paths.addons, addon_id)?)
    }
    Err(err) => {
      sp.finish_and_clear();
      return Err(err);
    }
  };
  if let Some(message) = install_result.update_message(addon_id) {
    println!("{message}");
  }
  let manifest = install_result.into_manifest();

  let mut lock = LockFile::load(project_root)?;

  for dep_id in &manifest.requires {
    if !is_addon_installed(&ctx.paths.addons, dep_id)? {
      return Err(anyhow!(
        "Addon '{}' requires '{}' to be installed first. Run: anesis addon install {}",
        addon_id,
        dep_id,
        dep_id
      ));
    }
  }

  let mut input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(&manifest.inputs, &mut input_values)?;

  let mut tera_ctx = tera::Context::new();
  insert_with_derived(&mut tera_ctx, &input_values);

  let detected_id = detect_variant(&manifest.detect, project_root);

  let variant = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()))
    .ok_or_else(|| anyhow!("No matching variant found for addon '{}'", addon_id))?;

  let command = variant
    .commands
    .iter()
    .find(|c| c.name == command_name)
    .ok_or_else(|| {
      anyhow!(
        "Command '{}' not found in addon '{}'",
        command_name,
        addon_id
      )
    })?;

  if command.once && lock.is_command_executed(addon_id, command_name) {
    if let Some(prompt_message) = rerun_prompt_message(
      command_name,
      lock.addon_version(addon_id),
      &manifest.version,
    ) {
      let rerun = Confirm::new(&prompt_message).with_default(false).prompt()?;
      if !rerun {
        println!("Skipping command '{}'.", command_name);
        return Ok(());
      }
    } else {
      println!(
        "Command '{}' has already been executed, skipping.",
        command_name
      );
      return Ok(());
    }
  }

  for req_cmd in &command.requires_commands {
    if !lock.is_command_executed(addon_id, req_cmd) {
      return Err(anyhow!(
        "Command '{}' requires '{}' to be run first. Run: anesis use {} {}",
        command_name,
        req_cmd,
        addon_id,
        req_cmd
      ));
    }
  }

  let mut cmd_input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(&command.inputs, &mut cmd_input_values)?;
  insert_with_derived(&mut tera_ctx, &cmd_input_values);

  if !confirm_addon_execution(addon_id, command_name, &command.steps)? {
    println!("Aborted. No changes were made.");
    return Ok(());
  }

  let addon_dir = ctx.paths.addons.join(addon_id);
  let mut completed_rollbacks: Vec<Rollback> = Vec::new();
  for (idx, step) in command.steps.iter().enumerate() {
    let result = match step {
      Step::Copy(s) => execute_copy(s, &addon_dir, project_root),
      Step::Create(s) => execute_create(s, project_root, &tera_ctx),
      Step::Inject(s) => execute_inject(s, project_root, &tera_ctx),
      Step::Replace(s) => execute_replace(s, project_root, &tera_ctx),
      Step::Append(s) => execute_append(s, project_root, &tera_ctx),
      Step::Delete(s) => execute_delete(s, project_root),
      Step::Rename(s) => execute_rename(s, project_root, &tera_ctx),
      Step::Move(s) => execute_move(s, project_root, &tera_ctx),
    };

    match result {
      Ok(rollbacks) => completed_rollbacks.extend(rollbacks),
      Err(err) => {
        eprintln!("Step {} failed: {}", idx + 1, err);
        let choice = Select::new(
          "How would you like to proceed?",
          vec!["Keep changes made so far", "Rollback all changes"],
        )
        .prompt()?;

        if choice == "Rollback all changes" {
          for rollback in completed_rollbacks.into_iter().rev() {
            let _ = apply_rollback(rollback, project_root);
          }
        }

        return Err(anyhow!("Addon command failed at step {}: {}", idx + 1, err));
      }
    }
  }

  let variant_id = detected_id.unwrap_or_else(|| "universal".to_string());
  let mut commands_executed = lock
    .addons
    .iter()
    .find(|e| e.id == addon_id)
    .map(|e| e.commands_executed.clone())
    .unwrap_or_default();
  if !commands_executed.iter().any(|c| c == command_name) {
    commands_executed.push(command_name.to_string());
  }
  lock.upsert_entry(LockEntry {
    id: addon_id.to_string(),
    version: manifest.version.clone(),
    variant: variant_id,
    commands_executed,
  });
  lock.save(project_root)?;

  record_addon_use(ctx, addon_id).await;

  if let Err(err) = AnesisManifest::add_addon(addon_id, project_root) {
    eprintln!("Note: could not update anesis.json ({err}).");
  }
  println!("✓ Command '{}' completed successfully.", command_name);
  Ok(())
}

fn confirm_addon_execution(addon_id: &str, command_name: &str, steps: &[Step]) -> Result<bool> {
  let (mut writes, mut edits, mut removes) = (0usize, 0usize, 0usize);
  for step in steps {
    match step {
      Step::Create(_) | Step::Copy(_) => writes += 1,
      Step::Inject(_) | Step::Replace(_) | Step::Append(_) => edits += 1,
      Step::Delete(_) | Step::Rename(_) | Step::Move(_) => removes += 1,
    }
  }

  println!(
    "⚠ Addon '{addon_id}' command '{command_name}' will modify files in this project \
     ({writes} created/copied, {edits} edited, {removes} deleted/moved)."
  );
  println!(
    "  Addons run unsandboxed and can overwrite source files or 'package.json'. \
     Only run addons you trust."
  );

  Ok(Confirm::new("Proceed?").with_default(false).prompt()?)
}

fn should_fallback_to_cached_manifest(error: &anyhow::Error) -> bool {
  if error.chain().any(|e| {
    e.downcast_ref::<AnesisError>().is_some_and(|e| {
      matches!(
        e,
        AnesisError::NotLoggedIn
          | AnesisError::HttpUnauthorized
          | AnesisError::NetworkTimeout
          | AnesisError::NetworkConnect
      )
    })
  }) {
    return true;
  }

  error.chain().any(|source| {
    source
      .downcast_ref::<reqwest::Error>()
      .is_some_and(|reqwest_error| {
        reqwest_error.is_connect()
          || reqwest_error.is_timeout()
          || reqwest_error.status().is_some_and(|status| {
            matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
          })
      })
  })
}

#[doc(hidden)]
pub fn should_fallback_to_cached_manifest_for_tests(error: &anyhow::Error) -> bool {
  should_fallback_to_cached_manifest(error)
}

fn rerun_prompt_message(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  let locked_version = locked_version.filter(|version| !version.is_empty())?;
  if locked_version == current_version {
    return None;
  }

  Some(format!(
    "Command '{}' was last run with v{} of this add-on. A new version (v{}) is available. Re-run it now?",
    command_name, locked_version, current_version
  ))
}

#[doc(hidden)]
pub fn rerun_prompt_message_for_tests(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  rerun_prompt_message(command_name, locked_version, current_version)
}

fn collect_inputs(inputs: &[InputDef], map: &mut HashMap<String, String>) -> Result<()> {
  for input in inputs {
    let value = match input.input_type {
      InputType::Text => {
        let mut prompt = Text::new(&input.description);
        if let Some(ref default) = input.default {
          prompt = prompt.with_default(default);
        }
        prompt.prompt()?
      }
      InputType::Boolean => {
        let default = input
          .default
          .as_deref()
          .map(|d| d == "true")
          .unwrap_or(false);
        Confirm::new(&input.description)
          .with_default(default)
          .prompt()?
          .to_string()
      }
      InputType::Select => Select::new(&input.description, input.options.clone())
        .prompt()?
        .to_string(),
    };
    map.insert(input.name.clone(), value);
  }
  Ok(())
}

fn insert_with_derived(ctx: &mut tera::Context, map: &HashMap<String, String>) {
  for (k, v) in map {
    ctx.insert(k.as_str(), v);
    ctx.insert(format!("{k}_pascal"), &to_pascal_case(v));
    ctx.insert(format!("{k}_camel"), &to_camel_case(v));
    ctx.insert(format!("{k}_kebab"), &to_kebab_case(v));
    ctx.insert(format!("{k}_snake"), &to_snake_case(v));
  }
}

fn apply_rollback(rollback: Rollback, project_root: &Path) -> Result<()> {
  match rollback {
    Rollback::DeleteCreatedFile { path } => {
      let _ = std::fs::remove_file(&path);
      let mut dir = path.parent();
      while let Some(d) = dir {
        if d == project_root {
          break;
        }

        if fs::remove_dir(d).is_err() {
          break;
        }
        dir = d.parent()
      }
    }
    Rollback::RestoreFile { path, original } => {
      std::fs::write(path, original)?;
    }
    Rollback::RenameFile { from, to } => {
      std::fs::rename(from, to)?;
    }
  }
  Ok(())
}
