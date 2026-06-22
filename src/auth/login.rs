use std::path::Path;

use anyhow::Result;
use inquire::Confirm;

use crate::{
  auth::{server::run_local_auth_server, token::get_auth_user},
  utils::ui::spinner,
};

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
  let user = run_local_auth_server(state, frontend_url)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let auth_json = serde_json::to_string(&user)?;
  write_auth_file(auth_path, &auth_json)?;

  println!("✅ Authorization successful as @{}", user.name);

  Ok(())
}

fn generate_state_token() -> String {
  uuid::Uuid::new_v4().simple().to_string()
}

#[doc(hidden)]
pub fn generate_state_token_for_tests() -> String {
  generate_state_token()
}

fn write_auth_file(path: &Path, content: &str) -> Result<()> {
  #[cfg(unix)]
  {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
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
