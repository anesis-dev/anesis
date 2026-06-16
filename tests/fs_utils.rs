use anesis::utils::fs::read_dir_to_files;
use assert_fs::prelude::*;

// ── read_dir_to_files ─────────────────────────────────────────────────────────

#[test]
fn read_flat_dir_returns_all_files() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").write_str("aaa").unwrap();
  dir.child("b.txt").write_str("bbb").unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  assert_eq!(files.len(), 2);
}

#[test]
fn read_flat_dir_relative_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("hi").unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  assert_eq!(files.len(), 1);
  assert!(!files[0].path.is_absolute(), "paths should be relative");
  assert_eq!(files[0].path.file_name().unwrap(), "hello.txt");
}

#[test]
fn read_flat_dir_captures_contents() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("data.txt").write_str("hello world").unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  assert_eq!(files[0].contents, b"hello world");
}

#[test]
fn read_nested_dir_finds_all_files() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("src")
    .child("index.ts")
    .write_str("export {}")
    .unwrap();
  dir
    .child("src")
    .child("utils")
    .child("helper.ts")
    .write_str("export {}")
    .unwrap();
  dir.child("README.md").write_str("# readme").unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  assert_eq!(files.len(), 3);
}

#[test]
fn read_nested_dir_returns_relative_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("src").child("index.ts").write_str("").unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  let path_str = files[0].path.to_string_lossy();
  assert!(
    path_str.contains("index.ts"),
    "expected index.ts in path, got: {path_str}"
  );
  assert!(!files[0].path.is_absolute());
}

#[test]
fn read_empty_dir_returns_empty_vec() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = read_dir_to_files(dir.path()).unwrap();
  assert!(files.is_empty());
}

#[test]
fn binary_file_contents_preserved() {
  let dir = assert_fs::TempDir::new().unwrap();
  let bytes: Vec<u8> = vec![0u8, 1, 2, 127, 255];
  dir.child("bin.dat").write_binary(&bytes).unwrap();

  let files = read_dir_to_files(dir.path()).unwrap();
  assert_eq!(files[0].contents, bytes);
}
