//! Shell tab-completion support.
//!
//! Two entry points:
//! - [`complete_env`] — called at process startup; if the `COMPLETE`
//!   environment variable is set (by the shell's completion hook), this
//!   prints the completion script and exits.
//! - [`install_completions`] — called by `anesis completions <shell>`; writes
//!   the generated script to the right directory and patches shell config
//!   files so it is sourced automatically.
//!
//! Completion candidates are generated from the local cache so installed
//! template and addon names appear as TAB completions without a network call.

use std::{
  collections::BTreeMap,
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
  process::Command as ProcessCommand,
};

use anyhow::{Context, Result};
use clap::{Command, ValueEnum};
use clap_complete::{
  engine::{ArgValueCandidates, CompletionCandidate},
  env::CompleteEnv,
};

use crate::{
  addons::{cache::AddonsCache, manifest::AddonManifest},
  cache::TemplatesCache,
  cli,
  paths::AnesisPaths,
};

/// Name of the environment variable that triggers completion mode.
/// Set by shell completion hooks (e.g. `complete -C "COMPLETE=zsh anesis" anesis`).
const COMPLETE_ENV_VAR: &str = "COMPLETE";
const INSTALLED_TEMPLATE_HELP: &str = "Installed template";
const INSTALLED_ADDON_HELP: &str = "Installed addon";
const INSTALLED_ADDON_COMMAND_HELP: &str = "Installed addon command";

/// Supported shells for tab-completion installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CompletionShell {
  Bash,
  Zsh,
  Fish,
  #[value(name = "powershell")]
  PowerShell,
}

impl CompletionShell {
  /// Returns the short name used as the value of the `COMPLETE` env var.
  fn env_name(self) -> &'static str {
    match self {
      Self::Bash => "bash",
      Self::Zsh => "zsh",
      Self::Fish => "fish",
      Self::PowerShell => "powershell",
    }
  }
}

/// Internal representation of an installed addon for completion generation.
#[derive(Clone, Debug, PartialEq, Eq)]
struct InstalledAddonCompletion {
  id: String,
  name: String,
  version: String,
  commands: Vec<InstalledAddonCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstalledAddonCommand {
  name: String,
  description: String,
}

/// If the `COMPLETE` env var is set, print shell completions and exit.
///
/// Must be called very early in `main` — before any argument parsing —
/// because `clap_complete` needs to intercept the raw argument list.
pub fn complete_env() {
  CompleteEnv::with_factory(command)
    .var(COMPLETE_ENV_VAR)
    .complete();
}

/// Generates and installs a completion script for the given shell.
pub fn install_completions(shell: CompletionShell) -> Result<()> {
  let script = generate_completion_script(shell)?;

  match shell {
    CompletionShell::Bash => install_bash(&script),
    CompletionShell::Zsh => install_zsh(&script),
    CompletionShell::Fish => install_fish(&script),
    CompletionShell::PowerShell => install_powershell(&script),
  }
}

/// Builds the clap [`Command`] tree augmented with dynamic completion
/// candidates read from the local cache.
///
/// Called both by [`complete_env`] (at runtime for completion) and by
/// [`install_completions`] (to generate the script).
pub fn command() -> Command {
  let paths = AnesisPaths::new().ok();
  command_for_paths(
    paths.as_ref().map(|p| p.templates.as_path()),
    paths.as_ref().map(|p| p.addons.as_path()),
  )
}

/// Same as [`command`] but accepts explicit cache directory paths, which
/// makes the logic unit-testable without touching `~/.anesis/`.
pub fn command_for_paths(templates_dir: Option<&Path>, addons_dir: Option<&Path>) -> Command {
  // Clone the paths into closures that are passed to `mut_subcommand`.
  // The closures need ownership of the PathBuf because they may outlive this
  // stack frame (clap stores them as boxed callbacks).
  let mut cmd = cli::command()
    .mut_subcommand("new", {
      let templates_dir = templates_dir.map(PathBuf::from);
      move |subcommand| {
        subcommand.mut_arg("template_name", {
          let templates_dir = templates_dir.clone();
          move |arg| {
            arg.add(ArgValueCandidates::new(move || {
              template_candidates(templates_dir.as_deref())
            }))
          }
        })
      }
    })
    .mut_subcommand("template", {
      let templates_dir = templates_dir.map(PathBuf::from);
      move |subcommand| {
        subcommand.mut_subcommand("remove", {
          let templates_dir = templates_dir.clone();
          move |remove| {
            remove.mut_arg("template_name", {
              let templates_dir = templates_dir.clone();
              move |arg| {
                arg.add(ArgValueCandidates::new(move || {
                  template_candidates(templates_dir.as_deref())
                }))
              }
            })
          }
        })
      }
    })
    .mut_subcommand("addon", {
      let addons_dir = addons_dir.map(PathBuf::from);
      move |subcommand| {
        subcommand.mut_subcommand("remove", {
          let addons_dir = addons_dir.clone();
          move |remove| {
            remove.mut_arg("addon_id", {
              let addons_dir = addons_dir.clone();
              move |arg| {
                arg.add(ArgValueCandidates::new(move || {
                  addon_candidates(addons_dir.as_deref())
                }))
              }
            })
          }
        })
      }
    });

  // Dynamically add `anesis use <addon-id> <command>` subcommands so the
  // shell can complete both the addon id and its available commands.
  if let Some(addons_dir) = addons_dir {
    let addons = installed_addons(addons_dir);
    if !addons.is_empty() {
      cmd = cmd.mut_subcommand("use", |use_cmd| {
        let mut use_cmd = use_cmd;
        for addon in addons {
          if use_cmd.find_subcommand(&addon.id).is_none() {
            use_cmd = use_cmd.subcommand(addon_subcommand(addon));
          }
        }
        use_cmd
      });
    }
  }

  cmd
}

/// Builds a clap subcommand for one installed addon under the `use` command.
fn addon_subcommand(addon: InstalledAddonCompletion) -> Command {
  let InstalledAddonCompletion {
    id,
    name,
    version,
    commands,
  } = addon;

  let mut subcommand = Command::new(id).about(format!("{INSTALLED_ADDON_HELP}: {name} v{version}"));

  for command in commands {
    let InstalledAddonCommand { name, description } = command;
    let mut addon_command = Command::new(name);

    addon_command = if description.is_empty() {
      addon_command.about(INSTALLED_ADDON_COMMAND_HELP)
    } else {
      addon_command.about(description)
    };

    subcommand = subcommand.subcommand(addon_command);
  }

  subcommand
}

/// Returns completion candidates for installed template names.
///
/// `templates_dir` is `None` when the cache root cannot be determined —
/// in that case an empty list is returned so completion degrades gracefully.
pub fn template_candidates(templates_dir: Option<&Path>) -> Vec<CompletionCandidate> {
  installed_template_names(templates_dir)
    .into_iter()
    .map(|name| CompletionCandidate::new(name).help(Some(INSTALLED_TEMPLATE_HELP.into())))
    .collect()
}

/// Returns completion candidates for installed addon IDs.
pub fn addon_candidates(addons_dir: Option<&Path>) -> Vec<CompletionCandidate> {
  let Some(addons_dir) = addons_dir else {
    return Vec::new();
  };

  installed_addons(addons_dir)
    .into_iter()
    .map(|addon| {
      CompletionCandidate::new(addon.id).help(Some(
        format!("{INSTALLED_ADDON_HELP}: {} v{}", addon.name, addon.version).into(),
      ))
    })
    .collect()
}

/// Reads template names from the local cache index, sorted and deduplicated.
fn installed_template_names(templates_dir: Option<&Path>) -> Vec<String> {
  let Some(templates_dir) = templates_dir else {
    return Vec::new();
  };

  let index = templates_dir.join("anesis-templates.json");
  // Silently ignore missing/corrupt cache — completion should never error.
  let Ok(content) = fs::read_to_string(&index) else {
    return Vec::new();
  };
  let Ok(cache) = serde_json::from_str::<TemplatesCache>(&content) else {
    return Vec::new();
  };

  let mut names: Vec<String> = cache
    .templates
    .into_iter()
    .map(|template| template.name)
    .collect();
  names.sort();
  names.dedup();
  names
}

/// Reads installed addons from the local cache index.
fn installed_addons(addons_dir: &Path) -> Vec<InstalledAddonCompletion> {
  let index = addons_dir.join("anesis-addons.json");
  let Ok(content) = fs::read_to_string(&index) else {
    return Vec::new();
  };
  let Ok(cache) = serde_json::from_str::<AddonsCache>(&content) else {
    return Vec::new();
  };

  let mut addons: Vec<InstalledAddonCompletion> = cache
    .addons
    .into_iter()
    .map(|addon| InstalledAddonCompletion {
      id: addon.id,
      name: addon.name,
      version: addon.version,
      commands: addon_commands(addons_dir, &addon.path),
    })
    .collect();
  addons.sort_by(|a, b| a.id.cmp(&b.id));
  addons
}

/// Reads the addon manifest and extracts unique command names with descriptions.
///
/// Uses `BTreeMap` to deduplicate commands across variants (an addon may
/// define the same command in multiple variants with slightly different steps).
/// The first non-empty description wins.
fn addon_commands(addons_dir: &Path, addon_path: &str) -> Vec<InstalledAddonCommand> {
  let manifest_path = addons_dir.join(addon_path).join("anesis.addon.json");
  let Ok(content) = fs::read_to_string(&manifest_path) else {
    return Vec::new();
  };
  let Ok(manifest) = serde_json::from_str::<AddonManifest>(&content) else {
    return Vec::new();
  };

  let mut commands: BTreeMap<String, String> = BTreeMap::new();

  for variant in manifest.variants {
    for command in variant.commands {
      commands
        .entry(command.name)
        .and_modify(|description| {
          // Keep the first non-empty description we encounter.
          if description.is_empty() && !command.description.is_empty() {
            *description = command.description.clone();
          }
        })
        .or_insert(command.description);
    }
  }

  commands
    .into_iter()
    .map(|(name, description)| InstalledAddonCommand { name, description })
    .collect()
}

/// Generates the completion script for the given shell by re-invoking the
/// current executable with `COMPLETE=<shell>` set.
///
/// `clap_complete` intercepts this invocation inside [`complete_env`] and
/// prints the script to stdout.
fn generate_completion_script(shell: CompletionShell) -> Result<String> {
  let current_exe = std::env::current_exe().context("Could not determine path to executable")?;
  let output = ProcessCommand::new(&current_exe)
    .env(COMPLETE_ENV_VAR, shell.env_name())
    .output()
    .with_context(|| {
      format!(
        "Could not generate {} completions via {}",
        shell.env_name(),
        current_exe.display()
      )
    })?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(anyhow::anyhow!(
      "Completion script generation for {} failed: {}",
      shell.env_name(),
      stderr.trim()
    ));
  }

  String::from_utf8(output.stdout).context("Generated completion script is not valid UTF-8")
}

// ── Bash ─────────────────────────────────────────────────────────────────────

fn install_bash(script: &str) -> Result<()> {
  let dir = bash_completions_dir()?;
  fs::create_dir_all(&dir)
    .with_context(|| format!("Could not create directory {}", dir.display()))?;
  let dest = dir.join("anesis");
  write_completion_script(&dest, script)?;
  println!("Written to {}", dest.display());
  println!(
    "\nTo activate, add this to your ~/.bashrc (if not already present):\n\
     \n  source ~/.local/share/bash-completion/completions/anesis\n\
     \nThen restart your shell or run:  source ~/.bashrc"
  );
  Ok(())
}

fn bash_completions_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".local/share/bash-completion/completions"))
}

// ── Zsh ──────────────────────────────────────────────────────────────────────

fn install_zsh(script: &str) -> Result<()> {
  if let Some(dir) = zdotdir_completions_dir() {
    // HyDE: completions directory is already in fpath — just drop the file.
    fs::create_dir_all(&dir)
      .with_context(|| format!("Could not create directory {}", dir.display()))?;
    let dest = dir.join("anesis.zsh");
    write_completion_script(&dest, script)?;
    println!("Written to {}", dest.display());
    println!("\nRestart your shell to activate completions.");
  } else {
    // Default: write to ~/.zfunc/_anesis and patch the zsh config file.
    let dir = home_zfunc_dir()?;
    fs::create_dir_all(&dir)
      .with_context(|| format!("Could not create directory {}", dir.display()))?;
    let dest = dir.join("_anesis");
    write_completion_script(&dest, script)?;

    let config = zsh_config_file()?;
    upsert_zsh_config(&config, &dir)?;

    println!("Written to {}", dest.display());
    println!("Updated {}", config.display());
    println!("\nRestart your shell or run:  source {}", config.display());
  }
  Ok(())
}

/// Returns the zsh config file to patch.
///
/// Preference order:
/// 1. `$ZDOTDIR/.zshrc`   — non-default ZDOTDIR
/// 2. `~/.zshrc`          — standard location (created if absent)
fn zsh_config_file() -> Result<PathBuf> {
  if let Ok(zdotdir) = std::env::var("ZDOTDIR") {
    let path = PathBuf::from(&zdotdir).join(".zshrc");
    return Ok(path);
  }

  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".zshrc"))
}

/// Inserts (or replaces) a managed block in the zsh config file that adds
/// `fpath_dir` to `fpath` and initialises the completion system.
pub fn upsert_zsh_config(config_path: &Path, fpath_dir: &Path) -> Result<()> {
  let existing = match fs::read_to_string(config_path) {
    Ok(content) => content,
    // Config file may not exist yet (fresh install); treat as empty.
    Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
    Err(err) => {
      return Err(err).with_context(|| format!("Could not read {}", config_path.display()));
    }
  };

  let snippet = zsh_fpath_snippet(fpath_dir);
  let updated = upsert_managed_block(
    &existing,
    &snippet,
    "# anesis completions start",
    "# anesis completions end",
  );

  if updated != existing {
    let dir = config_path
      .parent()
      .context("Zsh config path has no parent directory")?;
    fs::create_dir_all(dir)
      .with_context(|| format!("Could not create directory {}", dir.display()))?;
    fs::write(config_path, updated)
      .with_context(|| format!("Could not write {}", config_path.display()))?;
  }

  Ok(())
}

/// Generates the fpath snippet that should be added to `.zshrc`.
pub fn zsh_fpath_snippet(fpath_dir: &Path) -> String {
  let dir = fpath_dir.to_string_lossy();
  format!(
    "# anesis completions start\n\
fpath=({dir} $fpath)\n\
autoload -Uz compinit && compinit\n\
# anesis completions end"
  )
}

/// Returns the HyDE-specific completions directory if it exists and the user
/// appears to be running HyDE.  HyDE adds `$ZDOTDIR/completions` to fpath
/// automatically, so we can drop files there without patching config files.
fn zdotdir_completions_dir() -> Option<PathBuf> {
  let zdotdir = std::env::var("ZDOTDIR").map(PathBuf::from).ok()?;
  let dir = zdotdir.join("completions");
  if !dir.is_dir() {
    return None;
  }

  // Detect HyDE by checking for its config marker or CLI.
  let is_hyde = zdotdir.join(".hyde.zshrc").exists() || which::which("hyde-cli").is_ok();
  if is_hyde { Some(dir) } else { None }
}

fn home_zfunc_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".zfunc"))
}

// ── Fish ─────────────────────────────────────────────────────────────────────

fn install_fish(script: &str) -> Result<()> {
  let dir = fish_completions_dir()?;
  fs::create_dir_all(&dir)
    .with_context(|| format!("Could not create directory {}", dir.display()))?;
  let dest = dir.join("anesis.fish");
  write_completion_script(&dest, script)?;
  println!("Written to {}", dest.display());
  println!("\nRestart your shell to activate completions.");
  Ok(())
}

fn fish_completions_dir() -> Result<PathBuf> {
  // Prefer XDG_CONFIG_HOME if set; fall back to ~/.config.
  let config_dir = std::env::var("XDG_CONFIG_HOME")
    .map(PathBuf::from)
    .unwrap_or_else(|_| {
      dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".config")
    });
  Ok(config_dir.join("fish/completions"))
}

// ── PowerShell ────────────────────────────────────────────────────────────────

fn install_powershell(script: &str) -> Result<()> {
  let script_path = powershell_script_path()?;
  write_completion_script(&script_path, script)?;

  // Patch both PowerShell 5 (WindowsPowerShell) and PowerShell 7+ (PowerShell)
  // profiles so completions work regardless of which version the user runs.
  let profiles = powershell_profile_paths()?;
  for profile in &profiles {
    upsert_powershell_profile(profile, &script_path)?;
  }

  println!("Written to {}", script_path.display());
  println!("\nProfile updated. Restart PowerShell to activate completions.");
  Ok(())
}

fn powershell_script_path() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".anesis").join("completions").join("anesis.ps1"))
}

fn powershell_profile_paths() -> Result<Vec<PathBuf>> {
  let documents_dir = dirs::document_dir()
    .or_else(|| dirs::home_dir().map(|home| home.join("Documents")))
    .context("Could not determine Documents directory")?;
  Ok(powershell_profile_paths_in(&documents_dir))
}

/// Returns the standard profile paths for both PowerShell editions.
/// Extracted so tests can pass a temporary directory as `documents_dir`.
pub fn powershell_profile_paths_in(documents_dir: &Path) -> Vec<PathBuf> {
  vec![
    documents_dir
      .join("PowerShell")
      .join("Microsoft.PowerShell_profile.ps1"),
    documents_dir
      .join("WindowsPowerShell")
      .join("Microsoft.PowerShell_profile.ps1"),
  ]
}

/// Inserts (or replaces) the managed dot-source block in a PowerShell profile.
fn upsert_powershell_profile(profile_path: &Path, script_path: &Path) -> Result<()> {
  let dir = profile_path
    .parent()
    .context("PowerShell profile path has no parent directory")?;
  fs::create_dir_all(dir)
    .with_context(|| format!("Could not create directory {}", dir.display()))?;

  let existing = match fs::read_to_string(profile_path) {
    Ok(content) => content,
    Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
    Err(err) => {
      return Err(err).with_context(|| format!("Could not read {}", profile_path.display()));
    }
  };

  let updated = upsert_managed_block(
    &existing,
    &powershell_profile_snippet(script_path),
    "# anesis completions start",
    "# anesis completions end",
  );

  if updated != existing {
    fs::write(profile_path, updated)
      .with_context(|| format!("Could not write {}", profile_path.display()))?;
  }

  Ok(())
}

fn powershell_profile_snippet(script_path: &Path) -> String {
  let script_path = powershell_single_quote(script_path);
  format!(
    "# anesis completions start\n\
$anesisCompletionScript = '{script_path}'\n\
if (Test-Path $anesisCompletionScript) {{\n\
  . $anesisCompletionScript\n\
}}\n\
# anesis completions end"
  )
}

/// Escapes single quotes in a path for use inside PowerShell single-quoted strings.
fn powershell_single_quote(path: &Path) -> String {
  path.to_string_lossy().replace('\'', "''")
}

/// Idempotently inserts or replaces a marked block inside `content`.
///
/// If `start_marker` and `end_marker` are found, the entire block (including
/// both markers and the trailing newline) is replaced with `block`.  Otherwise
/// `block` is appended after a blank separator line.
///
/// This allows re-running `anesis completions` to update an existing block
/// without creating duplicates.
pub fn upsert_managed_block(
  content: &str,
  block: &str,
  start_marker: &str,
  end_marker: &str,
) -> String {
  // Normalise line endings so the logic works on Windows too.
  let mut content = content.replace("\r\n", "\n");
  let block = format!("{block}\n");

  if let Some(start) = content.find(start_marker)
    && let Some(end_rel) = content[start..].find(end_marker)
  {
    // Find the end of the line that contains end_marker to include its newline.
    let end_marker_end = start + end_rel + end_marker.len();
    let block_end = content[end_marker_end..]
      .find('\n')
      .map(|idx| end_marker_end + idx + 1)
      .unwrap_or(content.len());
    content.replace_range(start..block_end, &block);
    return content;
  }

  // No existing block — append after a blank line separator.
  if !content.is_empty() && !content.ends_with('\n') {
    content.push('\n');
  }
  if !content.is_empty() {
    content.push('\n');
  }
  content.push_str(&block);
  content
}

/// Writes `script` to `path`, creating parent directories as needed.
fn write_completion_script(path: &Path, script: &str) -> Result<()> {
  let dir = path
    .parent()
    .context("Completion file path has no parent directory")?;
  fs::create_dir_all(dir)
    .with_context(|| format!("Could not create directory {}", dir.display()))?;
  fs::write(path, script).with_context(|| format!("Could not write {}", path.display()))?;
  Ok(())
}
