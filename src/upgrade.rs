use std::{
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::{
  Client,
  header::{ACCEPT, USER_AGENT},
};
use serde::{Deserialize, Serialize};

use crate::{context::AppContext, utils::ui::spinner};

const RELEASES_API_URL: &str = "https://api.github.com/repos/anesis-dev/anesis/releases/latest";
const RELEASES_DOWNLOAD_BASE_URL: &str = "https://github.com/anesis-dev/anesis/releases/download";

#[derive(Debug, Deserialize)]
struct LatestReleaseResponse {
  tag_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct VersionCheckCache {
  last_checked: String,
  latest_version: String,
}

pub async fn check_latest_cli_version(client: &Client) -> Result<String> {
  let release: LatestReleaseResponse = client
    .get(releases_api_url())
    .header(ACCEPT, "application/vnd.github+json")
    .header(USER_AGENT, github_user_agent())
    .send()
    .await
    .context("Failed to query the latest Anesis release")?
    .error_for_status()
    .context("GitHub releases endpoint returned an error")?
    .json()
    .await
    .context("Failed to parse latest Anesis release metadata")?;

  normalize_version_tag(&release.tag_name)
}

pub async fn upgrade_cli(ctx: &AppContext) -> Result<()> {
  let current_version = env!("CARGO_PKG_VERSION");

  let sp = spinner("Checking for updates...");
  let latest_version = check_latest_cli_version(&ctx.client)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  if !is_newer_version(current_version, &latest_version)? {
    println!("Anesis v{current_version} is already the latest version.");
    return Ok(());
  }

  let platform = current_platform()?;
  let asset_url = release_asset_url(&latest_version, platform);
  let current_exe = env::current_exe().context("Failed to locate the current Anesis executable")?;

  let sp = spinner(format!("Downloading Anesis v{latest_version}..."));
  let archive_bytes = ctx
    .client
    .get(&asset_url)
    .header(USER_AGENT, github_user_agent())
    .send()
    .await
    .with_context(|| format!("Failed to download Anesis v{latest_version}"))
    .inspect_err(|_| sp.finish_and_clear())?
    .error_for_status()
    .with_context(|| format!("GitHub release asset was not available at {asset_url}"))
    .inspect_err(|_| sp.finish_and_clear())?
    .bytes()
    .await
    .with_context(|| format!("Failed to read the downloaded Anesis v{latest_version} archive"))
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  println!("Installing Anesis v{latest_version}...");
  let binary = extract_binary_from_archive(&archive_bytes, platform)
    .context("Failed to extract binary from downloaded archive")?;
  let temp_exe = write_temp_binary(&current_exe, &binary)?;
  mark_executable(&temp_exe)?;
  replace_current_executable(&current_exe, &temp_exe)?;

  println!("✓ Anesis updated to v{latest_version}. Restart your shell if needed.");
  Ok(())
}

pub async fn check_cli_version_cached(client: &Client, path: &Path) -> Result<Option<String>> {
  if let Some(cache) = read_version_check_cache(path)?
    && is_cache_fresh(&cache, Utc::now())
  {
    return newer_version_if_available(&cache.latest_version);
  }

  let latest_version = check_latest_cli_version(client).await?;
  write_version_check_cache(
    path,
    &VersionCheckCache {
      last_checked: Utc::now().to_rfc3339(),
      latest_version: latest_version.clone(),
    },
  )?;

  newer_version_if_available(&latest_version)
}

pub fn render_upgrade_notice(latest_version: &str) -> String {
  format!(
    "\n  A new version of Anesis is available: v{} → v{}\n  Run `anesis upgrade` to update.",
    env!("CARGO_PKG_VERSION"),
    latest_version
  )
}

fn github_user_agent() -> String {
  format!("anesis/{}", env!("CARGO_PKG_VERSION"))
}

fn releases_api_url() -> String {
  env::var("ANESIS_RELEASES_API_URL").unwrap_or_else(|_| RELEASES_API_URL.to_string())
}

fn releases_download_base_url() -> String {
  env::var("ANESIS_RELEASES_DOWNLOAD_BASE_URL")
    .unwrap_or_else(|_| RELEASES_DOWNLOAD_BASE_URL.to_string())
}

fn normalize_version_tag(tag_name: &str) -> Result<String> {
  let version = tag_name.strip_prefix('v').unwrap_or(tag_name);
  parse_version(version)?;
  Ok(version.to_string())
}

#[doc(hidden)]
pub fn normalize_version_tag_for_tests(tag_name: &str) -> Result<String> {
  normalize_version_tag(tag_name)
}

fn read_version_check_cache(path: &Path) -> Result<Option<VersionCheckCache>> {
  if !path.exists() {
    return Ok(None);
  }

  let content = fs::read_to_string(path)
    .with_context(|| format!("Failed to read version cache at {}", path.display()))?;
  let cache = match serde_json::from_str::<VersionCheckCache>(&content) {
    Ok(cache) => cache,
    Err(_) => return Ok(None),
  };
  Ok(Some(cache))
}

fn write_version_check_cache(path: &Path, cache: &VersionCheckCache) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).with_context(|| format!("Failed to create {}", parent.display()))?;
  }

  fs::write(path, serde_json::to_string_pretty(cache)?)
    .with_context(|| format!("Failed to write version cache to {}", path.display()))?;
  Ok(())
}

fn parse_version(version: &str) -> Result<(u64, u64, u64)> {
  let mut parts = version.split('.');
  let major = parse_version_component(parts.next(), "major", version)?;
  let minor = parse_version_component(parts.next(), "minor", version)?;
  let patch = parse_version_component(parts.next(), "patch", version)?;
  if parts.next().is_some() {
    return Err(anyhow!("Unsupported version format '{version}'"));
  }

  Ok((major, minor, patch))
}

#[doc(hidden)]
pub fn parse_version_for_tests(version: &str) -> Result<(u64, u64, u64)> {
  parse_version(version)
}

fn parse_version_component(component: Option<&str>, label: &str, version: &str) -> Result<u64> {
  let component =
    component.ok_or_else(|| anyhow!("Missing {label} version component in '{version}'"))?;
  component
    .parse::<u64>()
    .with_context(|| format!("Invalid {label} version component in '{version}'"))
}

fn is_newer_version(current: &str, latest: &str) -> Result<bool> {
  Ok(parse_version(latest)? > parse_version(current)?)
}

#[doc(hidden)]
pub fn is_newer_version_for_tests(current: &str, latest: &str) -> Result<bool> {
  is_newer_version(current, latest)
}

fn newer_version_if_available(latest: &str) -> Result<Option<String>> {
  if is_newer_version(env!("CARGO_PKG_VERSION"), latest)? {
    Ok(Some(latest.to_string()))
  } else {
    Ok(None)
  }
}

fn is_cache_fresh(cache: &VersionCheckCache, now: DateTime<Utc>) -> bool {
  let Ok(last_checked) = DateTime::parse_from_rfc3339(&cache.last_checked) else {
    return false;
  };

  now.signed_duration_since(last_checked.with_timezone(&Utc)) < ChronoDuration::hours(1)
}

#[doc(hidden)]
pub fn is_cache_fresh_for_tests(
  last_checked: &str,
  latest_version: &str,
  now: DateTime<Utc>,
) -> bool {
  is_cache_fresh(
    &VersionCheckCache {
      last_checked: last_checked.to_string(),
      latest_version: latest_version.to_string(),
    },
    now,
  )
}

fn current_platform() -> Result<&'static str> {
  if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Ok("linux-x86_64")
  } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Ok("macos-aarch64")
  } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Ok("windows-x86_64")
  } else {
    Err(anyhow!(
      "Unsupported platform for self-update: {}-{}",
      env::consts::OS,
      env::consts::ARCH
    ))
  }
}

fn asset_filename(platform: &str) -> String {
  if platform.starts_with("windows-") {
    format!("anesis-{platform}.zip")
  } else {
    format!("anesis-{platform}.tar.gz")
  }
}

fn extract_binary_from_archive(bytes: &[u8], platform: &str) -> Result<Vec<u8>> {
  if platform.starts_with("windows-") {
    extract_from_zip(bytes)
  } else {
    extract_from_targz(bytes)
  }
}

fn extract_from_targz(bytes: &[u8]) -> Result<Vec<u8>> {
  use flate2::read::GzDecoder;
  use std::io::Read;
  use tar::Archive;

  let gz = GzDecoder::new(bytes);
  let mut archive = Archive::new(gz);

  for entry in archive
    .entries()
    .context("Failed to read tar archive entries")?
  {
    let mut entry = entry.context("Failed to read tar archive entry")?;
    let path = entry.path().context("Failed to read entry path")?;
    let filename = path
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or("")
      .to_string();
    if filename == "anesis" {
      let mut binary = Vec::new();
      entry
        .read_to_end(&mut binary)
        .context("Failed to read binary from archive")?;
      return Ok(binary);
    }
  }

  Err(anyhow!("Binary 'anesis' not found in archive"))
}

fn extract_from_zip(bytes: &[u8]) -> Result<Vec<u8>> {
  use std::io::{Cursor, Read};
  use zip::ZipArchive;

  let cursor = Cursor::new(bytes);
  let mut archive = ZipArchive::new(cursor).context("Failed to read zip archive")?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i).context("Failed to read zip entry")?;
    let name = file.name().to_string();
    if name == "anesis.exe" || name == "anesis" {
      let mut binary = Vec::new();
      file
        .read_to_end(&mut binary)
        .context("Failed to read binary from zip")?;
      return Ok(binary);
    }
  }

  Err(anyhow!("Binary not found in zip archive"))
}

#[doc(hidden)]
pub fn asset_filename_for_tests(platform: &str) -> String {
  asset_filename(platform)
}

fn release_asset_url(version: &str, platform: &str) -> String {
  format!(
    "{}/v{version}/{}",
    releases_download_base_url(),
    asset_filename(platform)
  )
}

#[doc(hidden)]
pub fn release_asset_url_for_tests(version: &str, platform: &str) -> String {
  release_asset_url(version, platform)
}

fn write_temp_binary(current_exe: &Path, binary: &[u8]) -> Result<PathBuf> {
  let exe_dir = current_exe
    .parent()
    .ok_or_else(|| anyhow!("Failed to resolve the executable directory"))?;
  let exe_name = current_exe
    .file_name()
    .and_then(|name| name.to_str())
    .ok_or_else(|| anyhow!("Executable path is not valid UTF-8"))?;
  let temp_path = exe_dir.join(format!("{exe_name}.upgrade-{}.tmp", std::process::id()));
  fs::write(&temp_path, binary).with_context(|| {
    format!(
      "Failed to write downloaded binary to {}",
      temp_path.display()
    )
  })?;
  Ok(temp_path)
}

#[cfg(unix)]
fn mark_executable(path: &Path) -> Result<()> {
  use std::os::unix::fs::PermissionsExt;

  let mut permissions = fs::metadata(path)
    .with_context(|| format!("Failed to read permissions for {}", path.display()))?
    .permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions)
    .with_context(|| format!("Failed to mark {} as executable", path.display()))?;
  Ok(())
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) -> Result<()> {
  Ok(())
}

#[cfg(not(windows))]
fn replace_current_executable(current_exe: &Path, temp_exe: &Path) -> Result<()> {
  fs::rename(temp_exe, current_exe).with_context(|| {
    format!(
      "Failed to replace {} with {}",
      current_exe.display(),
      temp_exe.display()
    )
  })?;
  Ok(())
}

#[cfg(windows)]
fn replace_current_executable(current_exe: &Path, temp_exe: &Path) -> Result<()> {
  use std::process::Command;

  let updater_script =
    current_exe.with_file_name(format!("anesis-upgrade-{}.cmd", std::process::id()));
  let script = build_windows_updater_script(current_exe, temp_exe, &updater_script)?;
  fs::write(&updater_script, script)
    .with_context(|| format!("Failed to write {}", updater_script.display()))?;

  let updater_script = path_for_shell(&updater_script)?;
  Command::new("cmd")
    .args(["/C", "start", "", "/B", updater_script.as_str()])
    .spawn()
    .context("Failed to start the Windows updater helper")?;
  Ok(())
}

#[cfg(windows)]
fn build_windows_updater_script(
  current_exe: &Path,
  temp_exe: &Path,
  updater_script: &Path,
) -> Result<String> {
  let current_exe = quoted_windows_path(current_exe)?;
  let temp_exe = quoted_windows_path(temp_exe)?;
  let updater_script = quoted_windows_path(updater_script)?;
  Ok(format!(
    "@echo off\r\nping 127.0.0.1 -n 3 > nul\r\nmove /Y {temp_exe} {current_exe} > nul\r\ndel /Q {updater_script} > nul\r\n"
  ))
}

#[cfg(windows)]
fn quoted_windows_path(path: &Path) -> Result<String> {
  Ok(format!(
    "\"{}\"",
    path_for_shell(path)?.replace('"', "\"\"")
  ))
}

#[cfg(windows)]
fn path_for_shell(path: &Path) -> Result<String> {
  path
    .to_str()
    .map(ToOwned::to_owned)
    .ok_or_else(|| anyhow!("Path '{}' is not valid UTF-8", path.display()))
}
