//! `anesis login` implementation.
//!
//! Opens the user's browser to the backend's OAuth login page, then spins up
//! a one-shot local Axum server (see [`server`]) to receive the redirect
//! callback that carries the JWT.  The JWT is written to `~/.anesis/auth.json`
//! with restrictive permissions (0600 on Unix) so other local users cannot
//! read it.

use std::path::Path;

use anyhow::Result;
use inquire::Confirm;

use crate::{
  auth::{server::run_local_auth_server, token::get_auth_user},
  utils::ui::spinner,
};

/// Runs the interactive login flow.
///
/// If the user is already logged in, prompts before proceeding so an
/// accidental `anesis login` doesn't silently overwrite a valid session.
///
/// `auth_path`   — path to `~/.anesis/auth.json`
/// `backend_url` — base URL of the Anesis backend
/// `frontend_url` — base URL of the web frontend (used for post-auth redirects)
pub async fn login(auth_path: &Path, backend_url: &str, frontend_url: &str) -> Result<()> {
  if let Ok(existing) = get_auth_user(auth_path) {
    let proceed = Confirm::new(&format!(
      "Already logged in as @{}. Log in with a different account?",
      existing.name
    ))
    .with_default(false)
    .prompt()?;

    if !proceed {
      return Ok(());
    }
  }

  let state = generate_state_token();
  // NOTE: anesis-server must forward the `?state=` query param it receives
  // at /auth/cli-login through to the localhost callback redirect so that
  // the CSRF check below can validate it.
  let login_url = format!("{}/auth/cli-login?state={}", backend_url, state);
  if open::that(&login_url).is_err() {
    println!(
      "Could not open your browser automatically. Open this URL to log in:\n  {}",
      login_url
    );
  } else {
    println!("Opening browser for authorization...");
    println!("  {}", login_url);
  }
  let sp = spinner("Waiting for browser authorization...");
  // `run_local_auth_server` blocks until the callback arrives or times out.
  let user = run_local_auth_server(state, frontend_url)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let auth_json = serde_json::to_string(&user)?;
  write_auth_file(auth_path, &auth_json)?;

  println!("✅ Authorization successful as @{}", user.name);

  Ok(())
}

/// Generates a cryptographically random 128-bit state token for CSRF protection.
fn generate_state_token() -> String {
  uuid::Uuid::new_v4().simple().to_string()
}

#[doc(hidden)]
pub fn generate_state_token_for_tests() -> String {
  generate_state_token()
}

/// Writes `content` to `path` with owner-only read/write permissions (0600)
/// on Unix, preventing other local users from reading the auth token.
fn write_auth_file(path: &Path, content: &str) -> Result<()> {
  #[cfg(unix)]
  {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    // `OpenOptionsExt::mode` sets the Unix permission bits at file creation time.
    // `truncate(true)` ensures we overwrite any existing file rather than appending.
    let mut file = std::fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .mode(0o600)
      .open(path)?;
    file.write_all(content.as_bytes())?;
  }
  #[cfg(not(unix))]
  {
    std::fs::write(path, content)?;
  }
  Ok(())
}

#[doc(hidden)]
pub fn write_auth_file_for_tests(path: &Path, content: &str) -> Result<()> {
  write_auth_file(path, content)
}
