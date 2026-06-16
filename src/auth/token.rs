//! Reads the stored authentication credentials from disk.
//!
//! `~/.anesis/auth.json` contains a JSON-serialised [`User`] struct written
//! by the login flow.  Every command that requires authentication calls
//! [`get_auth_user`] to load and validate the token before making requests.

use std::{fs, path::Path};

use anyhow::Result;

use crate::{auth::server::User, utils::errors::AnesisError};

/// Loads and deserialises the stored [`User`] credentials.
///
/// Returns [`AnesisError::NotLoggedIn`] (via `anyhow::Error`) if the file is
/// absent or unreadable, so callers receive a user-friendly error message.
pub fn get_auth_user(auth_path: &Path) -> Result<User> {
  match fs::read_to_string(auth_path) {
    Ok(auth_json_str) => {
      let user: User = serde_json::from_str(&auth_json_str)?;
      Ok(user)
    }
    Err(_) => Err(AnesisError::NotLoggedIn.into()),
  }
}
