use std::{fs, path::Path};

use anyhow::Result;

use crate::{auth::server::User, utils::errors::AnesisError};

pub fn get_auth_user(auth_path: &Path) -> Result<User> {
  match fs::read_to_string(auth_path) {
    Ok(auth_json_str) => {
      let user: User = serde_json::from_str(&auth_json_str)?;
      Ok(user)
    }
    Err(_) => Err(AnesisError::NotLoggedIn.into()),
  }
}
