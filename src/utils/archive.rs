use std::{
  io::Cursor,
  path::{Component, Path},
};

use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use reqwest::Client;
use tar::Archive;

fn is_safe_relative(rel: &Path) -> bool {
  rel
    .components()
    .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

pub async fn download_and_extract(
  client: &Client,
  archive_url: &str,
  dest: &Path,
  subdir: Option<&str>,
  token: Option<&str>,
) -> Result<()> {
  let mut request = client.get(archive_url).header("User-Agent", "anesis");
  if let Some(token) = token {
    request = request.bearer_auth(token);
  }

  let bytes = request.send().await?.error_for_status()?.bytes().await?;

  std::fs::create_dir_all(dest)?;

  let gz = GzDecoder::new(Cursor::new(bytes));
  let mut archive = Archive::new(gz);

  for entry in archive.entries()? {
    let mut entry = entry?;
    let raw_path = entry.path()?.into_owned();

    let mut components = raw_path.components();
    components.next();
    let stripped = components.as_path();

    let rel = if let Some(dir) = subdir {
      match stripped.strip_prefix(dir) {
        Ok(r) => r.to_owned(),
        Err(_) => continue,
      }
    } else {
      stripped.to_owned()
    };

    if rel.as_os_str().is_empty() {
      continue;
    }

    if !is_safe_relative(&rel) {
      return Err(anyhow!(
        "refusing to extract entry with unsafe path: {}",
        rel.display()
      ));
    }

    let entry_type = entry.header().entry_type();
    if entry_type.is_symlink() || entry_type.is_hard_link() {
      continue;
    }

    let out_path = dest.join(&rel);
    if let Some(parent) = out_path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    entry.unpack(&out_path)?;
  }

  Ok(())
}

#[doc(hidden)]
pub fn strip_archive_path_for_tests(
  raw_path: &std::path::Path,
  subdir: Option<&str>,
) -> Option<std::path::PathBuf> {
  let mut components = raw_path.components();
  components.next();
  let stripped = components.as_path();

  let rel: std::path::PathBuf = if let Some(dir) = subdir {
    match stripped.strip_prefix(dir) {
      Ok(r) => r.to_owned(),
      Err(_) => return None,
    }
  } else {
    stripped.to_owned()
  };

  if rel.as_os_str().is_empty() {
    return None;
  }
  if !is_safe_relative(&rel) {
    return None;
  }
  Some(rel)
}
