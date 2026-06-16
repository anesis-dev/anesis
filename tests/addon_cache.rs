use anesis::addons::{
  cache::{
    get_cached_addon, get_installed_addons, is_addon_installed, remove_addon_from_cache,
    update_addons_cache,
  },
  manifest::AddonManifest,
};
use assert_fs::prelude::*;

fn make_manifest(id: &str) -> AddonManifest {
  AddonManifest {
    schema_version: "1".into(),
    id: id.into(),
    name: format!("{id}-name"),
    version: "1.0.0".into(),
    description: "test addon".into(),
    author: "test".into(),
    requires: vec![],
    inputs: vec![],
    detect: vec![],
    variants: vec![],
  }
}

// ── update_addons_cache ───────────────────────────────────────────────────────

#[test]
fn update_addons_cache_adds_entry() {
  let dir = assert_fs::TempDir::new().unwrap();
  let manifest = make_manifest("drizzle");

  update_addons_cache(dir.path(), "drizzle", &manifest, "sha1").unwrap();

  let result = get_cached_addon(dir.path(), "drizzle").unwrap();
  assert!(result.is_some());
  let entry = result.unwrap();
  assert_eq!(entry.id, "drizzle");
  assert_eq!(entry.version, "1.0.0");
  assert_eq!(entry.commit_sha, "sha1");
}

#[test]
fn update_addons_cache_replaces_duplicate() {
  let dir = assert_fs::TempDir::new().unwrap();
  let manifest = make_manifest("drizzle");

  update_addons_cache(dir.path(), "drizzle", &manifest, "sha1").unwrap();

  let mut manifest_v2 = make_manifest("drizzle");
  manifest_v2.version = "2.0.0".into();
  update_addons_cache(dir.path(), "drizzle", &manifest_v2, "sha2").unwrap();

  let result = get_cached_addon(dir.path(), "drizzle").unwrap().unwrap();
  assert_eq!(result.version, "2.0.0");
  assert_eq!(result.commit_sha, "sha2");

  // Only one entry after replacement
  let content = std::fs::read_to_string(dir.path().join("anesis-addons.json")).unwrap();
  let cache: anesis::addons::cache::AddonsCache = serde_json::from_str(&content).unwrap();
  assert_eq!(cache.addons.len(), 1);
}

#[test]
fn update_addons_cache_stores_multiple_addons() {
  let dir = assert_fs::TempDir::new().unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "sha1").unwrap();
  update_addons_cache(dir.path(), "prisma", &make_manifest("prisma"), "sha2").unwrap();

  let content = std::fs::read_to_string(dir.path().join("anesis-addons.json")).unwrap();
  let cache: anesis::addons::cache::AddonsCache = serde_json::from_str(&content).unwrap();
  assert_eq!(cache.addons.len(), 2);
}

// ── get_cached_addon ──────────────────────────────────────────────────────────

#[test]
fn get_cached_addon_returns_entry_when_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  let result = get_cached_addon(dir.path(), "drizzle").unwrap();
  assert!(result.is_some());
  assert_eq!(result.unwrap().id, "drizzle");
}

#[test]
fn get_cached_addon_returns_none_when_absent() {
  let dir = assert_fs::TempDir::new().unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  let result = get_cached_addon(dir.path(), "unknown").unwrap();
  assert!(result.is_none());
}

#[test]
fn get_cached_addon_returns_none_when_no_index() {
  let dir = assert_fs::TempDir::new().unwrap();
  let result = get_cached_addon(dir.path(), "drizzle").unwrap();
  assert!(result.is_none());
}

// ── remove_addon_from_cache ───────────────────────────────────────────────────

#[test]
fn remove_addon_removes_entry() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("drizzle")
    .child("anesis.addon.json")
    .write_str("{}")
    .unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  remove_addon_from_cache(dir.path(), "drizzle").unwrap();

  let result = get_cached_addon(dir.path(), "drizzle").unwrap();
  assert!(result.is_none());
}

#[test]
fn remove_addon_deletes_directory() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("drizzle")
    .child("anesis.addon.json")
    .write_str("{}")
    .unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  remove_addon_from_cache(dir.path(), "drizzle").unwrap();

  assert!(!dir.path().join("drizzle").exists());
}

#[test]
fn remove_addon_not_installed_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(remove_addon_from_cache(dir.path(), "nonexistent").is_err());
}

#[test]
fn remove_addon_keeps_other_addons() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("drizzle").child("file").write_str("").unwrap();
  dir.child("prisma").child("file").write_str("").unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "a").unwrap();
  update_addons_cache(dir.path(), "prisma", &make_manifest("prisma"), "b").unwrap();

  remove_addon_from_cache(dir.path(), "drizzle").unwrap();

  let result = get_cached_addon(dir.path(), "prisma").unwrap();
  assert!(result.is_some());
}

// ── is_addon_installed ────────────────────────────────────────────────────────

#[test]
fn is_addon_installed_true_when_in_cache_and_dir_exists() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("drizzle")
    .child("anesis.addon.json")
    .write_str("{}")
    .unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  assert!(is_addon_installed(dir.path(), "drizzle").unwrap());
}

#[test]
fn is_addon_installed_false_when_not_in_cache() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(!is_addon_installed(dir.path(), "drizzle").unwrap());
}

#[test]
fn is_addon_installed_false_when_dir_missing() {
  let dir = assert_fs::TempDir::new().unwrap();
  // Add to cache but don't create the directory
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();

  assert!(!is_addon_installed(dir.path(), "drizzle").unwrap());
}

// ── get_installed_addons ──────────────────────────────────────────────────────

#[test]
fn get_installed_addons_no_index_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(get_installed_addons(dir.path()).is_ok());
}

#[test]
fn get_installed_addons_with_addons_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  update_addons_cache(dir.path(), "drizzle", &make_manifest("drizzle"), "abc").unwrap();
  update_addons_cache(dir.path(), "prisma", &make_manifest("prisma"), "def").unwrap();

  assert!(get_installed_addons(dir.path()).is_ok());
}
