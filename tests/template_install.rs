mod common;

use anesis::{cache::CachedTemplate, templates::install::InstallResult};
use common::template_classify_install_state_for_tests;

fn cached_template(commit_sha: &str) -> CachedTemplate {
  CachedTemplate {
    name: "react-vite".to_string(),
    version: "1.0.0".to_string(),
    source: "https://github.com/example/react-vite".to_string(),
    path: "react-vite".to_string(),
    commit_sha: commit_sha.to_string(),
  }
}

#[test]
fn classify_install_state_returns_install_when_template_is_not_cached() {
  let install_state = template_classify_install_state_for_tests(None, true, "sha-1");
  assert_eq!(install_state, "install");
}

#[test]
fn classify_install_state_returns_install_when_directory_is_missing() {
  let cached_template = cached_template("sha-1");
  let install_state =
    template_classify_install_state_for_tests(Some(&cached_template), false, "sha-1");
  assert_eq!(install_state, "install");
}

#[test]
fn classify_install_state_returns_up_to_date_when_commit_matches() {
  let cached_template = cached_template("sha-1");
  let install_state =
    template_classify_install_state_for_tests(Some(&cached_template), true, "sha-1");
  assert_eq!(install_state, "up_to_date");
}

#[test]
fn classify_install_state_returns_update_when_commit_differs() {
  let cached_template = cached_template("sha-1");
  let install_state =
    template_classify_install_state_for_tests(Some(&cached_template), true, "sha-2");
  assert_eq!(install_state, "update");
}

#[test]
fn install_result_message_formats_update_message_with_version() {
  let message = InstallResult::Updated {
    version: "1.2.3".to_string(),
  }
  .message("react-vite");
  assert_eq!(
    message.as_deref(),
    Some("Template 'react-vite' updated to v1.2.3")
  );
}

#[test]
fn install_result_message_is_silent_for_up_to_date_templates() {
  let message = InstallResult::UpToDate.message("react-vite");
  assert!(message.is_none());
}
