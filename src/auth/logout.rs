use std::{fs, path::Path};

use anyhow::{Result, anyhow};

pub fn logout(auth_path: &Path) -> Result<()> {
  match fs::remove_file(auth_path) {
    Ok(_) => {
      println!("Logout successful");
      Ok(())
    }
    Err(_) => Err(anyhow!("You are not logged in yet.")),
  }
}
