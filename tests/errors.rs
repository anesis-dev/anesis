use std::fmt;

use anesis::utils::errors::AnesisError;

// ── AnesisError Display ────────────────────────────────────────────────────────

#[test]
fn not_logged_in_display() {
  let err = AnesisError::NotLoggedIn;
  assert_eq!(err.to_string(), "You are not logged in.");
}

#[test]
fn http_unauthorized_display() {
  let err = AnesisError::HttpUnauthorized;
  assert_eq!(
    err.to_string(),
    "Authentication failed. Your session may have expired."
  );
}

#[test]
fn http_not_found_display() {
  let err = AnesisError::HttpNotFound("template 'react-vite'".to_string());
  assert_eq!(err.to_string(), "template 'react-vite' was not found.");
}

#[test]
fn http_server_error_display() {
  let err = AnesisError::HttpServerError("template list".to_string());
  assert_eq!(
    err.to_string(),
    "The server returned an error while fetching template list."
  );
}

#[test]
fn network_connect_display() {
  let err = AnesisError::NetworkConnect;
  assert_eq!(err.to_string(), "Could not connect to the server.");
}

#[test]
fn network_timeout_display() {
  let err = AnesisError::NetworkTimeout;
  assert_eq!(err.to_string(), "The request timed out.");
}

// ── AnesisError in anyhow chain ────────────────────────────────────────────────

#[test]
fn anesis_error_wrapped_in_anyhow_is_downcastable() {
  let err: anyhow::Error = AnesisError::NotLoggedIn.into();
  assert!(err.downcast_ref::<AnesisError>().is_some());
}

#[test]
fn anesis_error_survives_anyhow_context_chain() {
  let err: anyhow::Error = AnesisError::HttpUnauthorized.into();
  let wrapped = err.context("fetching user account");
  let found = wrapped
    .chain()
    .any(|c| c.downcast_ref::<AnesisError>().is_some());
  assert!(
    found,
    "AnesisError should be discoverable in the error chain"
  );
}

// ── AnesisError is Debug + Display (both trait bounds required by the codebase) ──

#[test]
fn anesis_error_implements_debug() {
  let err = AnesisError::HttpNotFound("foo".into());
  let debug = format!("{err:?}");
  assert!(!debug.is_empty());
}

#[test]
fn all_variants_have_non_empty_messages() {
  let variants: Vec<Box<dyn fmt::Display>> = vec![
    Box::new(AnesisError::NotLoggedIn),
    Box::new(AnesisError::HttpUnauthorized),
    Box::new(AnesisError::HttpNotFound("x".into())),
    Box::new(AnesisError::HttpServerError("x".into())),
    Box::new(AnesisError::NetworkConnect),
    Box::new(AnesisError::NetworkTimeout),
  ];

  for v in variants {
    assert!(
      !v.to_string().is_empty(),
      "AnesisError variant has empty message: {v}"
    );
  }
}
