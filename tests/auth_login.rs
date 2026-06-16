mod common;

use assert_fs::prelude::*;
use common::{generate_state_token_for_tests, write_auth_file_for_tests};

// ── State token ───────────────────────────────────────────────────────────────

#[test]
fn state_token_is_32_hex_chars() {
  let token = generate_state_token_for_tests();
  assert_eq!(token.len(), 32, "UUID simple form should be 32 characters");
  assert!(
    token.chars().all(|c| c.is_ascii_hexdigit()),
    "token should be lowercase hex: {token}"
  );
}

#[test]
fn state_tokens_are_unique() {
  let t1 = generate_state_token_for_tests();
  let t2 = generate_state_token_for_tests();
  assert_ne!(t1, t2, "each call should produce a distinct token");
}

// ── Auth file writing ─────────────────────────────────────────────────────────

#[test]
fn write_auth_file_creates_file_with_correct_content() {
  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("auth.json");

  write_auth_file_for_tests(&path, r#"{"token":"x","name":"y"}"#).unwrap();

  assert!(path.exists());
  let content = std::fs::read_to_string(&path).unwrap();
  assert_eq!(content, r#"{"token":"x","name":"y"}"#);
}

#[test]
fn write_auth_file_overwrites_existing_content() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"old","name":"old"}"#)
    .unwrap();

  write_auth_file_for_tests(auth_file.path(), r#"{"token":"new","name":"new"}"#).unwrap();

  let content = std::fs::read_to_string(auth_file.path()).unwrap();
  assert_eq!(content, r#"{"token":"new","name":"new"}"#);
}

#[test]
fn write_auth_file_fails_when_parent_dir_missing() {
  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("nonexistent").join("auth.json");

  let err = write_auth_file_for_tests(&path, "{}").unwrap_err();
  assert!(
    err.to_string().contains("No such file") || err.to_string().contains("os error"),
    "expected I/O error, got: {err}"
  );
}

#[cfg(unix)]
#[test]
fn write_auth_file_sets_owner_only_permissions() {
  use std::os::unix::fs::MetadataExt;

  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("auth.json");

  write_auth_file_for_tests(&path, r#"{"token":"t","name":"n"}"#).unwrap();

  let meta = std::fs::metadata(&path).unwrap();
  assert_eq!(
    meta.mode() & 0o777,
    0o600,
    "auth file must be readable/writable only by owner"
  );
}
