use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::{archive::download_and_extract, errors::classify_reqwest_error, ui::spinner},
};

use super::{
  cache::{CachedAddon, get_cached_addon, update_addons_cache},
  manifest::AddonManifest,
};

pub async fn record_addon_use(ctx: &AppContext, addon_id: &str) {
  let Ok(user) = get_auth_user(&ctx.paths.auth) else {
    return;
  };
  let url = format!("{}/addon/{}/use", ctx.backend_url, addon_id);
  if let Err(e) = ctx
    .client
    .post(&url)
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await
  {
    log::warn!("Failed to record addon use event: {e:?}");
  }
}

#[derive(Deserialize)]
struct AddonUrlResponse {
  archive_url: String,
  #[serde(default)]
  archive_token: Option<String>,
  commit_sha: String,
  subdir: Option<String>,
}

#[derive(Debug)]
pub enum AddonInstallResult {
  Installed(AddonManifest),
  Updated(AddonManifest),
  UpToDate(AddonManifest),
}

impl AddonInstallResult {
  pub fn into_manifest(self) -> AddonManifest {
    match self {
      Self::Installed(manifest) | Self::Updated(manifest) | Self::UpToDate(manifest) => manifest,
    }
  }

  pub fn message(&self, addon_id: &str) -> Option<String> {
    match self {
      Self::Installed(_) => Some(format!("Addon '{addon_id}' successfully downloaded")),
      Self::Updated(manifest) => Some(format!(
        "Addon '{addon_id}' updated to v{}",
        manifest.version
      )),
      Self::UpToDate(_) => None,
    }
  }

  pub fn update_message(&self, addon_id: &str) -> Option<String> {
    match self {
      Self::Updated(manifest) => Some(format!(
        "Addon '{addon_id}' updated to v{}",
        manifest.version
      )),
      Self::Installed(_) | Self::UpToDate(_) => None,
    }
  }

  pub fn up_to_date_message(addon_id: &str) -> String {
    format!("Addon '{addon_id}' is already up to date")
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallState {
  Install,
  Update,
  UpToDate,
}

fn classify_install_state(
  cached_addon: Option<&CachedAddon>,
  addon_dir_exists: bool,
  latest_commit_sha: &str,
) -> InstallState {
  let Some(cached_addon) = cached_addon else {
    return InstallState::Install;
  };

  if !addon_dir_exists {
    return InstallState::Install;
  }

  if cached_addon.commit_sha == latest_commit_sha {
    InstallState::UpToDate
  } else {
    InstallState::Update
  }
}

#[doc(hidden)]
pub fn classify_install_state_for_tests(
  cached_addon: Option<&CachedAddon>,
  addon_dir_exists: bool,
  latest_commit_sha: &str,
) -> &'static str {
  match classify_install_state(cached_addon, addon_dir_exists, latest_commit_sha) {
    InstallState::Install => "install",
    InstallState::Update => "update",
    InstallState::UpToDate => "up_to_date",
  }
}

async fn get_addon_url(ctx: &AppContext, addon_id: &str) -> Result<AddonUrlResponse> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let response = ctx
    .client
    .get(format!("{}/addon/{addon_id}/url", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await
    .with_context(|| format!("Failed to fetch download URL for addon '{addon_id}'"))?;

  if !response.status().is_success() {
    let err = response.error_for_status().unwrap_err();
    return Err(classify_reqwest_error(err, &format!("addon '{addon_id}'")));
  }

  let res: AddonUrlResponse = response
    .json()
    .await
    .with_context(|| format!("Failed to parse URL response for addon '{addon_id}'"))?;

  Ok(res)
}

pub async fn install_addon(ctx: &AppContext, addon_id: &str) -> Result<AddonInstallResult> {
  let sp = spinner(format!("Fetching info for addon '{addon_id}'..."));
  let info = get_addon_url(ctx, addon_id)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let addons_dir = &ctx.paths.addons;
  let addon_dir = addons_dir.join(addon_id);
  let cached_addon = get_cached_addon(addons_dir, addon_id)
    .with_context(|| format!("Failed to read addons cache while checking '{addon_id}'"))?;
  let install_state =
    classify_install_state(cached_addon.as_ref(), addon_dir.exists(), &info.commit_sha);

  if install_state == InstallState::UpToDate {
    return Ok(AddonInstallResult::UpToDate(read_cached_manifest(
      addons_dir, addon_id,
    )?));
  }

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(addon_dir.clone());
  }

  if install_state == InstallState::Update && addon_dir.exists() {
    std::fs::remove_dir_all(&addon_dir)
      .with_context(|| format!("Failed to clear stale addon at {}", addon_dir.display()))?;
  }

  let action = if install_state == InstallState::Update {
    "Updating"
  } else {
    "Downloading"
  };
  let sp = spinner(format!("{action} addon '{addon_id}'..."));
  let download_result = download_and_extract(
    &ctx.client,
    &info.archive_url,
    &addon_dir,
    info.subdir.as_deref(),
    info.archive_token.as_deref(),
  )
  .await
  .with_context(|| {
    format!(
      "Failed to download and extract addon '{addon_id}' from {}",
      info.archive_url
    )
  });
  sp.finish_and_clear();

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  download_result?;

  let manifest_path = addon_dir.join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "Addon '{addon_id}' was extracted but 'anesis.addon.json' was not found at {}. \
       Make sure the addon archive contains a top-level '{addon_id}/' directory with 'anesis.addon.json' inside.",
      manifest_path.display()
    )
  })?;

  let manifest: AddonManifest = serde_json::from_str(&content)
    .with_context(|| format!("Failed to parse anesis.addon.json for addon '{addon_id}'"))?;

  update_addons_cache(addons_dir, addon_id, &manifest, &info.commit_sha)
    .with_context(|| format!("Failed to update addons cache after installing '{addon_id}'"))?;

  Ok(match install_state {
    InstallState::Install => AddonInstallResult::Installed(manifest),
    InstallState::Update => AddonInstallResult::Updated(manifest),
    InstallState::UpToDate => unreachable!("up-to-date addons should return early"),
  })
}

pub fn read_cached_manifest(addons_dir: &Path, addon_id: &str) -> Result<AddonManifest> {
  let manifest_path = addons_dir.join(addon_id).join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "Failed to read manifest for addon '{addon_id}' at {}",
      manifest_path.display()
    )
  })?;
  serde_json::from_str(&content)
    .with_context(|| format!("Failed to parse manifest for addon '{addon_id}'"))
}
