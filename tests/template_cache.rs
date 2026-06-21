use anesis::{
  cache::{
    TemplatesCache, get_cached_template, get_installed_templates, is_template_installed,
    remove_template_from_cache, update_templates_cache,
  },
  context::AppContext,
  paths::AnesisPaths,
};
use assert_fs::prelude::*;

fn make_test_ctx(templates_dir: &std::path::Path) -> AppContext {
  let paths = AnesisPaths {
    home: templates_dir.to_path_buf(),
    config: templates_dir.join("config.json"),
    version_check: templates_dir.join("version_check.json"),
    cache: templates_dir.join("cache"),
    templates: templates_dir.to_path_buf(),
    auth: templates_dir.join("auth.json"),
    addons: templates_dir.join("cache").join("addons"),
    addons_index: templates_dir
      .join("cache")
      .join("addons")
      .join("anesis-addons.json"),
  };
  AppContext {
    paths,
    client: reqwest::Client::new(),
    cleanup_state: std::sync::Arc::new(std::sync::Mutex::new(None)),
    backend_url: String::new(),
    frontend_url: String::new(),
  }
}

fn write_anesis_template_json(dir: &assert_fs::TempDir, subdir: &str, name: &str) {
  let json = serde_json::json!({
    "name": name,
    "version": "1.0.0",
    "anesisVersion": "0.5.0",
    "repository": { "url": "https://github.com/example/repo" },
    "metadata": { "displayName": name, "description": "test" }
  });
  dir
    .child(subdir)
    .child("anesis.template.json")
    .write_str(&json.to_string())
    .unwrap();
}

#[test]
fn update_cache_adds_entry() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");

  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc123").unwrap();

  let content = std::fs::read_to_string(dir.path().join("anesis-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert_eq!(cache.templates.len(), 1);
  assert_eq!(cache.templates[0].name, "react-vite");
  assert_eq!(cache.templates[0].commit_sha, "abc123");
}

#[test]
fn update_cache_replaces_duplicate() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");

  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "aaa").unwrap();
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "bbb").unwrap();

  let content = std::fs::read_to_string(dir.path().join("anesis-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert_eq!(cache.templates.len(), 1);
  assert_eq!(cache.templates[0].commit_sha, "bbb");
}

#[test]
fn remove_template_removes_entry_and_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();
  dir
    .child("react-vite")
    .child("index.js")
    .write_str("")
    .unwrap();

  remove_template_from_cache(dir.path(), "react-vite").unwrap();

  let content = std::fs::read_to_string(dir.path().join("anesis-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert!(cache.templates.is_empty());
  assert!(!dir.path().join("react-vite").exists());
}

#[test]
fn remove_template_not_installed_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();

  assert!(remove_template_from_cache(dir.path(), "nonexistent").is_err());
}

#[test]
fn get_installed_templates_no_file_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(get_installed_templates(dir.path()).is_ok());
}

#[test]
fn get_installed_templates_with_entries_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();

  assert!(get_installed_templates(dir.path()).is_ok());
}

// ── get_cached_template ───────────────────────────────────────────────────────

#[test]
fn get_cached_template_returns_entry_when_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "sha1").unwrap();

  let ctx = make_test_ctx(dir.path());
  let result = get_cached_template(&ctx, "react-vite").unwrap();

  assert!(result.is_some());
  let entry = result.unwrap();
  assert_eq!(entry.name, "react-vite");
  assert_eq!(entry.commit_sha, "sha1");
}

#[test]
fn get_cached_template_returns_none_for_unknown_name() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "sha1").unwrap();

  let ctx = make_test_ctx(dir.path());
  let result = get_cached_template(&ctx, "nonexistent").unwrap();

  assert!(result.is_none());
}

#[test]
fn get_cached_template_returns_none_when_no_index() {
  let dir = assert_fs::TempDir::new().unwrap();
  let ctx = make_test_ctx(dir.path());
  let result = get_cached_template(&ctx, "react-vite").unwrap();
  assert!(result.is_none());
}

// ── is_template_installed ─────────────────────────────────────────────────────

#[test]
fn is_template_installed_true_when_cached_and_dir_exists() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();

  let ctx = make_test_ctx(dir.path());
  assert!(is_template_installed(&ctx, "react-vite").unwrap());
}

#[test]
fn is_template_installed_false_when_not_in_cache() {
  let dir = assert_fs::TempDir::new().unwrap();
  let ctx = make_test_ctx(dir.path());
  assert!(!is_template_installed(&ctx, "react-vite").unwrap());
}

#[test]
fn is_template_installed_false_when_dir_missing() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_anesis_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();
  // Remove the directory after caching
  std::fs::remove_dir_all(dir.path().join("react-vite")).unwrap();

  let ctx = make_test_ctx(dir.path());
  assert!(!is_template_installed(&ctx, "react-vite").unwrap());
}
