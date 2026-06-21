use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;

use crate::context::CleanupState;

pub fn setup_ctrlc_handler(
  cleanup_state: CleanupState,
  template_path_clone: PathBuf,
) -> Result<()> {
  ctrlc::set_handler(move || {
    println!("\n⚠ Interrupted! Cleaning up...");

    let cleanup_path = {
      let guard = cleanup_state.lock().unwrap_or_else(|e| e.into_inner());

      guard.clone()
    };
    if let Some(path) = cleanup_path
      && path.exists()
    {
      if let Err(e) = fs::remove_dir_all(&path) {
        eprintln!("Failed to remove: {}", e);
      }

      let mut current = path.parent();
      while let Some(parent) = current {
        if parent == template_path_clone {
          break;
        }
        if fs::remove_dir(parent).is_err() {
          break;
        }
        current = parent.parent();
      }
      println!("✓ Removed incomplete template");
    }
    std::process::exit(1);
  })?;

  Ok(())
}

#[doc(hidden)]
pub fn cleanup_incomplete_template_for_tests(cleanup_path: &Path, template_root: &Path) {
  if !cleanup_path.exists() {
    return;
  }
  let _ = fs::remove_dir_all(cleanup_path);
  let mut current = cleanup_path.parent();
  while let Some(parent) = current {
    if parent == template_root {
      break;
    }
    if fs::remove_dir(parent).is_err() {
      break;
    }
    current = parent.parent();
  }
}
