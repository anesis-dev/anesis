use anesis::addons::lock::{LockEntry, LockFile};

fn sample_entry(id: &str) -> LockEntry {
  LockEntry {
    id: id.to_string(),
    version: "1.0.0".to_string(),
    variant: "universal".to_string(),
    commands_executed: vec![],
  }
}

#[test]
fn load_returns_default_when_no_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let lock = LockFile::load(dir.path()).unwrap();
  assert!(lock.addons.is_empty());
}

#[test]
fn save_and_load_roundtrip() {
  let dir = assert_fs::TempDir::new().unwrap();
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.mark_command_executed("drizzle", "install");
  lock.save(dir.path()).unwrap();

  let loaded = LockFile::load(dir.path()).unwrap();
  assert_eq!(loaded.addons.len(), 1);
  assert_eq!(loaded.addons[0].id, "drizzle");
  assert!(loaded.is_command_executed("drizzle", "install"));
}

#[test]
fn is_command_executed_false_when_no_entry() {
  let lock = LockFile::default();
  assert!(!lock.is_command_executed("drizzle", "install"));
}

#[test]
fn addon_version_returns_current_version_when_entry_exists() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addon_version("drizzle"), Some("1.0.0"));
}

#[test]
fn mark_command_executed_adds_once() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.mark_command_executed("drizzle", "install");
  lock.mark_command_executed("drizzle", "install");
  let entry = lock.addons.iter().find(|e| e.id == "drizzle").unwrap();
  assert_eq!(entry.commands_executed.len(), 1);
}

#[test]
fn upsert_entry_adds_new() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  assert_eq!(lock.addons.len(), 1);
}

#[test]
fn upsert_entry_replaces_existing() {
  let mut lock = LockFile::default();
  lock.upsert_entry(sample_entry("drizzle"));
  lock.upsert_entry(LockEntry {
    id: "drizzle".to_string(),
    version: "2.0.0".to_string(),
    variant: "nestjs".to_string(),
    commands_executed: vec!["install".to_string()],
  });
  assert_eq!(lock.addons.len(), 1);
  assert_eq!(lock.addons[0].version, "2.0.0");
  assert_eq!(lock.addons[0].variant, "nestjs");
}

#[test]
fn mark_command_executed_noop_when_no_entry() {
  let mut lock = LockFile::default();
  lock.mark_command_executed("unknown", "install");
  assert!(lock.addons.is_empty());
}
