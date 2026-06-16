//! `anesis logout` implementation.
//!
//! Logout is intentionally simple: deleting `~/.anesis/auth.json` is enough
//! because all authentication state is local.  There is no server-side session
//! to invalidate — the JWT is simply abandoned.

use std::{fs, path::Path};

use anyhow::{Result, anyhow};

/// Removes the stored auth credentials file.
///
/// Returns an error if the file does not exist, which means the user was
/// never logged in.
pub fn logout(auth_path: &Path) -> Result<()> {
  match fs::remove_file(auth_path) {
    Ok(_) => {
      println!("Logout successful");
      Ok(())
    }
    Err(_) => Err(anyhow!("You are not logged in yet.")),
  }
}
