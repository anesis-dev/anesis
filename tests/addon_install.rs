mod common;

use anesis::addons::{cache::CachedAddon, install::AddonInstallResult, manifest::AddonManifest};
use common::addon_classify_install_state_for_tests;

fn cached_addon(commit_sha: &str) -> CachedAddon {
  CachedAddon {
    id: "drizzle".to_string(),
    name: "Drizzle".to_string(),
    version: "1.0.0".to_string(),
    path: "drizzle".to_string(),
    commit_sha: commit_sha.to_string(),
    repo_url: String::new(),
  }
}

fn manifest(version: &str) -> AddonManifest {
  AddonManifest {
    schema_version: "1".to_string(),
    id: "drizzle".to_string(),
    name: "Drizzle".to_string(),
    version: version.to_string(),
    description: "test addon".to_string(),
    author: "test".to_string(),
    requires: Vec::new(),
    inputs: Vec::new(),
    detect: Vec::new(),
    variants: Vec::new(),
  }
}

#[test]
fn classify_install_state_returns_install_when_addon_is_not_cached() {
  let install_state = addon_classify_install_state_for_tests(None, true, "sha-1");
  assert_eq!(install_state, "install");
}

#[test]
fn classify_install_state_returns_install_when_addon_directory_is_missing() {
  let cached_addon = cached_addon("sha-1");
  let install_state = addon_classify_install_state_for_tests(Some(&cached_addon), false, "sha-1");
  assert_eq!(install_state, "install");
}

#[test]
fn classify_install_state_returns_up_to_date_when_commit_matches() {
  let cached_addon = cached_addon("sha-1");
  let install_state = addon_classify_install_state_for_tests(Some(&cached_addon), true, "sha-1");
  assert_eq!(install_state, "up_to_date");
}

#[test]
fn classify_install_state_returns_update_when_commit_differs() {
  let cached_addon = cached_addon("sha-1");
  let install_state = addon_classify_install_state_for_tests(Some(&cached_addon), true, "sha-2");
  assert_eq!(install_state, "update");
}

#[test]
fn addon_install_result_update_message_uses_new_version() {
  let message = AddonInstallResult::Updated(manifest("1.2.3")).update_message("drizzle");
  assert_eq!(
    message.as_deref(),
    Some("Addon 'drizzle' updated to v1.2.3")
  );
}

#[test]
fn addon_install_result_message_is_silent_for_up_to_date_addons() {
  let message = AddonInstallResult::UpToDate(manifest("1.2.3")).message("drizzle");
  assert!(message.is_none());
}
