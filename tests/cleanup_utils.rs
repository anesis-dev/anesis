mod common;

use common::cleanup_incomplete_template_for_tests;

#[test]
fn removes_target_directory() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let target = templates_dir.join("my-template");
  std::fs::create_dir_all(&target).unwrap();
  std::fs::write(target.join("file.txt"), "content").unwrap();

  cleanup_incomplete_template_for_tests(&target, &templates_dir);

  assert!(!target.exists(), "target directory should be removed");
}

#[test]
fn removes_empty_parent_directories_up_to_template_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  // nested: templates/group/my-template
  let group = templates_dir.join("group");
  let target = group.join("my-template");
  std::fs::create_dir_all(&target).unwrap();

  cleanup_incomplete_template_for_tests(&target, &templates_dir);

  assert!(!target.exists());
  // "group" is empty after removing "my-template", so it should also be gone
  assert!(!group.exists(), "empty intermediate dir should be removed");
  // templates_dir itself should NOT be removed (it's the root boundary)
  assert!(
    templates_dir.exists(),
    "template root should not be removed"
  );
}

#[test]
fn stops_removing_parents_when_dir_is_not_empty() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let group = templates_dir.join("group");
  let sibling = group.join("other-template");
  let target = group.join("my-template");
  std::fs::create_dir_all(&target).unwrap();
  std::fs::create_dir_all(&sibling).unwrap(); // sibling keeps "group" non-empty

  cleanup_incomplete_template_for_tests(&target, &templates_dir);

  assert!(!target.exists());
  // "group" still has "other-template" inside, so it must not be removed
  assert!(group.exists(), "non-empty parent should not be removed");
  assert!(sibling.exists(), "sibling directory must be untouched");
}

#[test]
fn is_noop_when_target_does_not_exist() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  std::fs::create_dir_all(&templates_dir).unwrap();
  let target = templates_dir.join("nonexistent");

  // Should not panic or error even when the path is missing.
  cleanup_incomplete_template_for_tests(&target, &templates_dir);
}

#[test]
fn removes_nested_files_inside_target() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let target = templates_dir.join("half-done");
  let nested = target.join("src").join("index.ts");
  std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
  std::fs::write(&nested, "export {}").unwrap();

  cleanup_incomplete_template_for_tests(&target, &templates_dir);

  assert!(!target.exists());
}
