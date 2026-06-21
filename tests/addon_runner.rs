mod common;

use anesis::utils::errors::AnesisError;
use anyhow::anyhow;
use common::{rerun_prompt_message_for_tests, should_fallback_to_cached_manifest_for_tests};

#[test]
fn fallback_to_cached_manifest_when_user_is_not_logged_in() {
  let error = anyhow::Error::from(AnesisError::NotLoggedIn);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn do_not_fallback_to_cached_manifest_for_unrelated_errors() {
  let error = anyhow!("anesis.addon.json is malformed");
  assert!(!should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn rerun_prompt_message_is_none_when_versions_match() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.0.0");
  assert!(prompt.is_none());
}

#[test]
fn rerun_prompt_message_mentions_both_versions_when_version_changed() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.1.0");
  assert_eq!(
    prompt.as_deref(),
    Some(
      "Command 'install' was last run with v1.0.0 of this add-on. A new version (v1.1.0) is available. Re-run it now?"
    )
  );
}

#[test]
fn rerun_prompt_message_is_none_when_no_prior_version_recorded() {
  // None means the command has never been executed → no re-run prompt needed.
  let prompt = rerun_prompt_message_for_tests("install", None, "1.0.0");
  assert!(
    prompt.is_none(),
    "should not prompt to re-run on a fresh install"
  );
}

#[test]
fn should_fallback_for_http_unauthorized() {
  let error = anyhow::Error::from(AnesisError::HttpUnauthorized);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn should_fallback_for_network_connect_anesis_error() {
  // A connection failure is a transient/offline condition, so the runner falls
  // back to the cached manifest rather than aborting the command.
  let error = anyhow::Error::from(AnesisError::NetworkConnect);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn should_fallback_for_network_timeout_anesis_error() {
  let error = anyhow::Error::from(AnesisError::NetworkTimeout);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn should_not_fallback_for_http_server_error() {
  let error = anyhow::Error::from(AnesisError::HttpServerError("addon".to_string()));
  assert!(!should_fallback_to_cached_manifest_for_tests(&error));
}
