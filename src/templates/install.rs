use std::path::Path;

use anyhow::{Context, Result};
use log::debug;
use reqwest::Client;
use serde::Deserialize;

use crate::{
  auth::token::get_auth_user,
  cache::{CachedTemplate, get_cached_template, update_templates_cache},
  context::AppContext,
  utils::{archive::download_and_extract, errors::classify_reqwest_error, ui::spinner},
};

pub async fn record_template_use(ctx: &AppContext, template_name: &str) {
  let Ok(user) = get_auth_user(&ctx.paths.auth) else {
    return;
  };
  let url = format!("{}/template/{}/use", ctx.backend_url, template_name);
  if let Err(e) = ctx
    .client
    .post(&url)
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await
  {
    log::warn!("Failed to record template use event: {e:?}");
  }
}

#[derive(Deserialize)]
struct TemplateInfoRes {
  archive_url: String,
  #[serde(default)]
  archive_token: Option<String>,
  commit_sha: String,
  subdir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallResult {
  Installed,
  Updated { version: String },
  UpToDate,
}

impl InstallResult {
  pub fn message(&self, template_name: &str) -> Option<String> {
    match self {
      Self::Installed => Some(format!(
        "Template '{template_name}' downloaded successfully"
      )),
      Self::Updated { version } => {
        Some(format!("Template '{template_name}' updated to v{version}"))
      }
      Self::UpToDate => None,
    }
  }

  pub fn up_to_date_message(template_name: &str) -> String {
    format!("Template '{template_name}' is already up to date")
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallState {
  Install,
  Update,
  UpToDate,
}

fn classify_install_state(
  cached_template: Option<&CachedTemplate>,
  template_dir_exists: bool,
  latest_commit_sha: &str,
) -> InstallState {
  let Some(cached_template) = cached_template else {
    return InstallState::Install;
  };

  if !template_dir_exists {
    return InstallState::Install;
  }

  if cached_template.commit_sha == latest_commit_sha {
    InstallState::UpToDate
  } else {
    InstallState::Update
  }
}

#[doc(hidden)]
pub fn classify_install_state_for_tests(
  cached_template: Option<&CachedTemplate>,
  template_dir_exists: bool,
  latest_commit_sha: &str,
) -> &'static str {
  match classify_install_state(cached_template, template_dir_exists, latest_commit_sha) {
    InstallState::Install => "install",
    InstallState::Update => "update",
    InstallState::UpToDate => "up_to_date",
  }
}

async fn get_template_info(
  template_name: &str,
  client: &Client,
  auth_path: &Path,
  backend_url: &str,
) -> Result<TemplateInfoRes> {
  let user = get_auth_user(auth_path)?;

  let response = client
    .get(format!("{backend_url}/template/{template_name}/url"))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await
    .with_context(|| format!("Failed to connect to server for template '{template_name}'"))?;

  if !response.status().is_success() {
    let err = response.error_for_status().unwrap_err();
    return Err(classify_reqwest_error(
      err,
      &format!("template '{template_name}'"),
    ));
  }

  let res: TemplateInfoRes = response
    .json()
    .await
    .with_context(|| format!("Failed to parse response for template '{template_name}'"))?;

  Ok(res)
}

pub async fn install_template(ctx: &AppContext, template_name: &str) -> Result<InstallResult> {
  let sp = spinner(format!("Fetching info for template '{template_name}'..."));
  let info = get_template_info(
    template_name,
    &ctx.client,
    &ctx.paths.auth,
    &ctx.backend_url,
  )
  .await
  .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let dest = ctx.paths.templates.join(template_name);
  debug!("Checking cache for template '{template_name}'");
  let cached_template = get_cached_template(ctx, template_name)?;
  debug!("Cached template: {:?}", cached_template);
  let install_state =
    classify_install_state(cached_template.as_ref(), dest.exists(), &info.commit_sha);
  debug!("Install state: {:?}", install_state);

  if install_state == InstallState::UpToDate {
    return Ok(InstallResult::UpToDate);
  }

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(dest.clone());
  }

  if install_state == InstallState::Update && dest.exists() {
    std::fs::remove_dir_all(&dest)
      .with_context(|| format!("Failed to clear stale template at {}", dest.display()))?;
  }

  let action = if install_state == InstallState::Update {
    "Updating"
  } else {
    "Downloading"
  };
  let sp = spinner(format!("{action} template '{template_name}'..."));
  debug!("Start download files");
  let download_result = download_and_extract(
    &ctx.client,
    &info.archive_url,
    &dest,
    info.subdir.as_deref(),
    info.archive_token.as_deref(),
  )
  .await;
  debug!("End download files");
  sp.finish_and_clear();

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  download_result?;

  debug!("Start caching template");
  let cached_template = update_templates_cache(
    &ctx.paths.templates,
    Path::new(template_name),
    &info.commit_sha,
  )?;
  debug!("End caching template: {:?}", cached_template);

  Ok(match install_state {
    InstallState::Install => InstallResult::Installed,
    InstallState::Update => InstallResult::Updated {
      version: cached_template.version,
    },
    InstallState::UpToDate => unreachable!("up-to-date templates should return early"),
  })
}
