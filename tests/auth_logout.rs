use anesis::auth::logout::logout;
use assert_fs::prelude::*;

#[test]
fn removes_auth_file_when_logged_in() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"tok","name":"alice"}"#)
    .unwrap();

  logout(auth_file.path()).unwrap();

  assert!(!auth_file.path().exists(), "auth file should be deleted");
}

#[test]
fn returns_error_when_not_logged_in() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_path = dir.path().join("nonexistent.json");

  let err = logout(&auth_path).unwrap_err();
  assert!(
    err.to_string().contains("not logged in"),
    "expected 'not logged in' message, got: {err}"
  );
}

#[test]
fn logout_is_idempotent_failure() {
  // Calling logout twice should fail the second time.
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file.write_str("{}").unwrap();

  logout(auth_file.path()).unwrap();
  let err = logout(auth_file.path()).unwrap_err();
  assert!(err.to_string().contains("not logged in"));
}
