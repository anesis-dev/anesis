
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

const COMPLETE_ENV_VAR: &str = "COMPLETE";
const INSTALLED_TEMPLATE_HELP: &str = "Installed template";
const INSTALLED_ADDON_HELP: &str = "Installed addon";
const INSTALLED_ADDON_COMMAND_HELP: &str = "Installed addon command";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CompletionShell {
  Bash,
  Zsh,
  Fish,
  #[value(name = "powershell")]
  PowerShell,
}

impl CompletionShell {
  fn env_name(self) -> &'static str {
    match self {
      Self::Bash => "bash",
      Self::Zsh => "zsh",
      Self::Fish => "fish",
      Self::PowerShell => "powershell",
    }
  }
}

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

pub fn complete_env() {
  CompleteEnv::with_factory(command)
    .var(COMPLETE_ENV_VAR)
    .complete();
}

pub fn install_completions(shell: CompletionShell) -> Result<()> {
  let script = generate_completion_script(shell)?;

  match shell {
    CompletionShell::Bash => install_bash(&script),
    CompletionShell::Zsh => install_zsh(&script),
    CompletionShell::Fish => install_fish(&script),
    CompletionShell::PowerShell => install_powershell(&script),
  }
}

pub fn command() -> Command {
  let paths = AnesisPaths::new().ok();
  command_for_paths(
    paths.as_ref().map(|p| p.templates.as_path()),
    paths.as_ref().map(|p| p.addons.as_path()),
  )
}

pub fn command_for_paths(templates_dir: Option<&Path>, addons_dir: Option<&Path>) -> Command {
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

pub fn template_candidates(templates_dir: Option<&Path>) -> Vec<CompletionCandidate> {
  installed_template_names(templates_dir)
    .into_iter()
    .map(|name| CompletionCandidate::new(name).help(Some(INSTALLED_TEMPLATE_HELP.into())))
    .collect()
}

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

fn installed_template_names(templates_dir: Option<&Path>) -> Vec<String> {
  let Some(templates_dir) = templates_dir else {
    return Vec::new();
  };

  let index = templates_dir.join("anesis-templates.json");
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

fn install_zsh(script: &str) -> Result<()> {
  if let Some(dir) = zdotdir_completions_dir() {
    fs::create_dir_all(&dir)
      .with_context(|| format!("Could not create directory {}", dir.display()))?;
    let dest = dir.join("anesis.zsh");
    write_completion_script(&dest, script)?;
    println!("Written to {}", dest.display());
    println!("\nRestart your shell to activate completions.");
  } else {
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

fn zsh_config_file() -> Result<PathBuf> {
  if let Ok(zdotdir) = std::env::var("ZDOTDIR") {
    let path = PathBuf::from(&zdotdir).join(".zshrc");
    return Ok(path);
  }

  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".zshrc"))
}

pub fn upsert_zsh_config(config_path: &Path, fpath_dir: &Path) -> Result<()> {
  let existing = match fs::read_to_string(config_path) {
    Ok(content) => content,
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

pub fn zsh_fpath_snippet(fpath_dir: &Path) -> String {
  let dir = fpath_dir.to_string_lossy();
  format!(
    "# anesis completions start\n\
fpath=({dir} $fpath)\n\
autoload -Uz compinit && compinit\n\
# anesis completions end"
  )
}

fn zdotdir_completions_dir() -> Option<PathBuf> {
  let zdotdir = std::env::var("ZDOTDIR").map(PathBuf::from).ok()?;
  let dir = zdotdir.join("completions");
  if !dir.is_dir() {
    return None;
  }

  let is_hyde = zdotdir.join(".hyde.zshrc").exists() || which::which("hyde-cli").is_ok();
  if is_hyde { Some(dir) } else { None }
}

fn home_zfunc_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not determine home directory")?;
  Ok(home.join(".zfunc"))
}

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
  let config_dir = std::env::var("XDG_CONFIG_HOME")
    .map(PathBuf::from)
    .unwrap_or_else(|_| {
      dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".config")
    });
  Ok(config_dir.join("fish/completions"))
}

fn install_powershell(script: &str) -> Result<()> {
  let script_path = powershell_script_path()?;
  write_completion_script(&script_path, script)?;

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

fn powershell_single_quote(path: &Path) -> String {
  path.to_string_lossy().replace('\'', "''")
}

pub fn upsert_managed_block(
  content: &str,
  block: &str,
  start_marker: &str,
  end_marker: &str,
) -> String {
  let mut content = content.replace("\r\n", "\n");
  let block = format!("{block}\n");

  if let Some(start) = content.find(start_marker)
    && let Some(end_rel) = content[start..].find(end_marker)
  {
    let end_marker_end = start + end_rel + end_marker.len();
    let block_end = content[end_marker_end..]
      .find('\n')
      .map(|idx| end_marker_end + idx + 1)
      .unwrap_or(content.len());
    content.replace_range(start..block_end, &block);
    return content;
  }

  if !content.is_empty() && !content.ends_with('\n') {
    content.push('\n');
  }
  if !content.is_empty() {
    content.push('\n');
  }
  content.push_str(&block);
  content
}

fn write_completion_script(path: &Path, script: &str) -> Result<()> {
  let dir = path
    .parent()
    .context("Completion file path has no parent directory")?;
  fs::create_dir_all(dir)
    .with_context(|| format!("Could not create directory {}", dir.display()))?;
  fs::write(path, script).with_context(|| format!("Could not write {}", path.display()))?;
  Ok(())
}
