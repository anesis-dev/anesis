use anesis::addons::{
  manifest::{
    AppendStep, CopyStep, CreateStep, DeleteStep, IfExists, IfNotFound, InjectStep, MoveStep,
    RenameStep, ReplaceStep, Target,
  },
  steps::{
    Rollback, append::execute_append, copy::execute_copy, create::execute_create,
    delete::execute_delete, inject::execute_inject, move_step::execute_move,
    rename::execute_rename, render_lines, render_string, replace::execute_replace,
  },
};
use assert_fs::prelude::*;

fn empty_ctx() -> tera::Context {
  tera::Context::new()
}

fn ctx_with(key: &str, val: &str) -> tera::Context {
  let mut c = tera::Context::new();
  c.insert(key, val);
  c
}

// ── create ────────────────────────────────────────────────────────────────────

#[test]
fn create_new_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "Hello, {{ name }}!".into(),
    if_exists: IfExists::Overwrite,
  };
  let rollbacks = execute_create(&step, dir.path(), &ctx_with("name", "world")).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "Hello, world!");
  assert!(matches!(rollbacks[0], Rollback::DeleteCreatedFile { .. }));
}

#[test]
fn create_overwrites_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("old content").unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "new content".into(),
    if_exists: IfExists::Overwrite,
  };
  let rollbacks = execute_create(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "new content");
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn create_skips_existing_file_when_skip() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("hello.txt").write_str("original").unwrap();
  let step = CreateStep {
    path: "hello.txt".into(),
    content: "new content".into(),
    if_exists: IfExists::Skip,
  };
  let rollbacks = execute_create(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
  assert_eq!(content, "original");
  assert!(rollbacks.is_empty());
}

#[test]
fn create_nested_dirs() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "src/components/Button.tsx".into(),
    content: "export default function Button() {}".into(),
    if_exists: IfExists::Overwrite,
  };
  execute_create(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(dir.path().join("src/components/Button.tsx").exists());
}

// ── inject ────────────────────────────────────────────────────────────────────

#[test]
fn inject_after_marker() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("// imports\nconst app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "import cors from 'cors';".into(),
    after: Some("// imports".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let result = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  let lines: Vec<&str> = result.lines().collect();
  assert_eq!(lines[0], "// imports");
  assert_eq!(lines[1], "import cors from 'cors';");
  assert_eq!(lines[2], "const app = express();");
}

#[test]
fn inject_before_marker() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "// header".into(),
    after: None,
    before: Some("const app".into()),
    if_not_found: IfNotFound::Error,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let lines: Vec<String> = std::fs::read_to_string(dir.path().join("app.ts"))
    .unwrap()
    .lines()
    .map(str::to_string)
    .collect();
  assert_eq!(lines[0], "// header");
  assert_eq!(lines[1], "const app = express();");
}

#[test]
fn inject_marker_not_found_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line".into(),
    after: Some("// nonexistent".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  assert!(execute_inject(&step, dir.path(), &empty_ctx()).is_err());
}

#[test]
fn inject_marker_not_found_skip() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.ts")
    .write_str("const app = express();")
    .unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line".into(),
    after: Some("// nonexistent".into()),
    before: None,
    if_not_found: IfNotFound::Skip,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const app = express();");
}

#[test]
fn inject_no_marker_prepends() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("line2").unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "line1".into(),
    after: None,
    before: None,
    if_not_found: IfNotFound::Skip,
  };

  execute_inject(&step, dir.path(), &empty_ctx()).unwrap();

  let lines: Vec<String> = std::fs::read_to_string(dir.path().join("app.ts"))
    .unwrap()
    .lines()
    .map(str::to_string)
    .collect();
  assert_eq!(lines[0], "line1");
  assert_eq!(lines[1], "line2");
}

// ── replace ───────────────────────────────────────────────────────────────────

#[test]
fn replace_substitutes_text() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "3000".into(),
    replace: "4000".into(),
    if_not_found: IfNotFound::Error,
  };

  execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const PORT = 4000;");
}

#[test]
fn replace_not_found_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "9999".into(),
    replace: "0".into(),
    if_not_found: IfNotFound::Error,
  };

  assert!(execute_replace(&step, dir.path(), &empty_ctx()).is_err());
}

#[test]
fn replace_not_found_skip_leaves_file_unchanged() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("const PORT = 3000;").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "9999".into(),
    replace: "0".into(),
    if_not_found: IfNotFound::Skip,
  };

  execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert_eq!(content, "const PORT = 3000;");
}

#[test]
fn replace_rollback_is_restore_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("original").unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    find: "original".into(),
    replace: "replaced".into(),
    if_not_found: IfNotFound::Error,
  };

  let rollbacks = execute_replace(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

// ── append ────────────────────────────────────────────────────────────────────

#[test]
fn appends_to_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "line2".into(),
  };

  execute_append(&step, dir.path(), &empty_ctx()).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  let lines: Vec<&str> = content.lines().collect();
  assert_eq!(lines[0], "line1");
  assert_eq!(lines[1], "line2");
}

#[test]
fn append_rollback_is_restore_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("original").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "appended".into(),
  };

  let rollbacks = execute_append(&step, dir.path(), &empty_ctx()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

// ── delete ────────────────────────────────────────────────────────────────────

#[test]
fn deletes_existing_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("remove-me.txt").write_str("bye").unwrap();

  let step = DeleteStep {
    target: Target::File {
      file: "remove-me.txt".into(),
    },
  };
  execute_delete(&step, dir.path()).unwrap();

  assert!(!dir.path().join("remove-me.txt").exists());
}

#[test]
fn delete_rollback_stores_original_bytes() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("important data").unwrap();

  let step = DeleteStep {
    target: Target::File {
      file: "file.txt".into(),
    },
  };
  let rollbacks = execute_delete(&step, dir.path()).unwrap();

  match &rollbacks[0] {
    Rollback::RestoreFile { original, .. } => assert_eq!(original, b"important data"),
    _ => panic!("expected RestoreFile rollback"),
  }
}

#[test]
fn delete_nonexistent_file_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = DeleteStep {
    target: Target::File {
      file: "ghost.txt".into(),
    },
  };
  let rollbacks = execute_delete(&step, dir.path()).unwrap();
  assert!(rollbacks.is_empty());
}

// ── rename ────────────────────────────────────────────────────────────────────

#[test]
fn renames_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("old.txt").write_str("data").unwrap();

  let step = RenameStep {
    from: "old.txt".into(),
    to: "new.txt".into(),
  };
  execute_rename(&step, dir.path(), &tera::Context::new()).unwrap();

  assert!(!dir.path().join("old.txt").exists());
  assert!(dir.path().join("new.txt").exists());
}

#[test]
fn rename_rollback_reverses() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("old.txt").write_str("data").unwrap();

  let step = RenameStep {
    from: "old.txt".into(),
    to: "new.txt".into(),
  };
  let rollbacks = execute_rename(&step, dir.path(), &tera::Context::new()).unwrap();

  match &rollbacks[0] {
    Rollback::RenameFile { from, to } => {
      assert_eq!(from, &dir.path().join("new.txt"));
      assert_eq!(to, &dir.path().join("old.txt"));
    }
    _ => panic!("expected RenameFile rollback"),
  }
}

#[test]
fn rename_source_missing_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = RenameStep {
    from: "ghost.txt".into(),
    to: "new.txt".into(),
  };
  assert!(execute_rename(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn rename_target_exists_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").write_str("a").unwrap();
  dir.child("b.txt").write_str("b").unwrap();

  let step = RenameStep {
    from: "a.txt".into(),
    to: "b.txt".into(),
  };
  assert!(execute_rename(&step, dir.path(), &tera::Context::new()).is_err());
}

// ── move ──────────────────────────────────────────────────────────────────────

#[test]
fn moves_file_to_new_directory() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "file.txt".into(),
    to: "subdir/file.txt".into(),
  };
  execute_move(&step, dir.path(), &tera::Context::new()).unwrap();

  assert!(!dir.path().join("file.txt").exists());
  assert!(dir.path().join("subdir/file.txt").exists());
}

#[test]
fn move_creates_destination_dirs() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "file.txt".into(),
    to: "a/b/c/file.txt".into(),
  };
  execute_move(&step, dir.path(), &tera::Context::new()).unwrap();
  assert!(dir.path().join("a/b/c/file.txt").exists());
}

#[test]
fn move_source_missing_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = MoveStep {
    from: "ghost.txt".into(),
    to: "dest.txt".into(),
  };
  assert!(execute_move(&step, dir.path(), &tera::Context::new()).is_err());
}

#[test]
fn move_target_exists_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").write_str("a").unwrap();
  dir.child("b.txt").write_str("b").unwrap();

  let step = MoveStep {
    from: "a.txt".into(),
    to: "b.txt".into(),
  };
  assert!(execute_move(&step, dir.path(), &tera::Context::new()).is_err());
}

// ── copy ──────────────────────────────────────────────────────────────────────

#[test]
fn copy_creates_new_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("template.txt").write_str("hello").unwrap();

  let step = CopyStep {
    src: "template.txt".into(),
    dest: "output.txt".into(),
    if_exists: IfExists::Overwrite,
  };

  execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();

  let content = std::fs::read_to_string(project_dir.path().join("output.txt")).unwrap();
  assert_eq!(content, "hello");
}

#[test]
fn copy_new_file_rollback_is_delete_created() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("content").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Overwrite,
  };

  let rollbacks = execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::DeleteCreatedFile { .. }));
}

#[test]
fn copy_overwrites_existing_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir
    .child("template.txt")
    .write_str("new content")
    .unwrap();
  project_dir
    .child("output.txt")
    .write_str("old content")
    .unwrap();

  let step = CopyStep {
    src: "template.txt".into(),
    dest: "output.txt".into(),
    if_exists: IfExists::Overwrite,
  };

  execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();

  let content = std::fs::read_to_string(project_dir.path().join("output.txt")).unwrap();
  assert_eq!(content, "new content");
}

#[test]
fn copy_overwrite_rollback_is_restore_file() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("new").unwrap();
  project_dir.child("dst.txt").write_str("original").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Overwrite,
  };

  let rollbacks = execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();
  assert!(matches!(rollbacks[0], Rollback::RestoreFile { .. }));
}

#[test]
fn copy_skip_leaves_existing_file_unchanged() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("new content").unwrap();
  project_dir.child("dst.txt").write_str("original").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "dst.txt".into(),
    if_exists: IfExists::Skip,
  };

  let rollbacks = execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();
  assert!(rollbacks.is_empty());

  let content = std::fs::read_to_string(project_dir.path().join("dst.txt")).unwrap();
  assert_eq!(content, "original");
}

#[test]
fn copy_creates_destination_subdirectory() {
  let addon_dir = assert_fs::TempDir::new().unwrap();
  let project_dir = assert_fs::TempDir::new().unwrap();
  addon_dir.child("src.txt").write_str("data").unwrap();

  let step = CopyStep {
    src: "src.txt".into(),
    dest: "subdir/dst.txt".into(),
    if_exists: IfExists::Overwrite,
  };

  execute_copy(&step, addon_dir.path(), project_dir.path()).unwrap();
  assert!(project_dir.path().join("subdir/dst.txt").exists());
}

// ── render_lines ──────────────────────────────────────────────────────────────

#[test]
fn render_lines_substitutes_variables() {
  let mut ctx = tera::Context::new();
  ctx.insert("name", "world");
  let lines = vec![
    "Hello, {{ name }}!".to_string(),
    "Goodbye, {{ name }}.".to_string(),
  ];
  let rendered = render_lines(&lines, &ctx).unwrap();
  assert_eq!(rendered[0], "Hello, world!");
  assert_eq!(rendered[1], "Goodbye, world.");
}

#[test]
fn render_lines_empty_input_returns_empty() {
  let ctx = tera::Context::new();
  let rendered = render_lines(&[], &ctx).unwrap();
  assert!(rendered.is_empty());
}

#[test]
fn render_lines_no_variables_unchanged() {
  let ctx = tera::Context::new();
  let lines = vec!["plain line".to_string()];
  let rendered = render_lines(&lines, &ctx).unwrap();
  assert_eq!(rendered[0], "plain line");
}

// ── render_string ─────────────────────────────────────────────────────────────

#[test]
fn render_string_substitutes_variable() {
  let mut ctx = tera::Context::new();
  ctx.insert("env", "production");
  let result = render_string(".env.{{ env }}", &ctx).unwrap();
  assert_eq!(result, ".env.production");
}

#[test]
fn render_string_no_variable_unchanged() {
  let ctx = tera::Context::new();
  let result = render_string("plain-string", &ctx).unwrap();
  assert_eq!(result, "plain-string");
}

#[test]
fn render_string_multiple_variables() {
  let mut ctx = tera::Context::new();
  ctx.insert("prefix", "my");
  ctx.insert("suffix", "app");
  let result = render_string("{{ prefix }}-{{ suffix }}", &ctx).unwrap();
  assert_eq!(result, "my-app");
}

// ── path traversal via create ─────────────────────────────────────────────────

#[test]
fn create_path_traversal_blocked() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = CreateStep {
    path: "../../etc/passwd".into(),
    content: "evil".into(),
    if_exists: IfExists::Overwrite,
  };
  assert!(execute_create(&step, dir.path(), &empty_ctx()).is_err());
}

// ── template variables inside step content ────────────────────────────────────

#[test]
fn inject_content_uses_template_vars() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.ts").write_str("// imports\n").unwrap();

  let step = InjectStep {
    target: Target::File {
      file: "app.ts".into(),
    },
    content: "import {{ lib }} from '{{ lib }}';".into(),
    after: Some("// imports".into()),
    before: None,
    if_not_found: IfNotFound::Error,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("lib", "cors");
  execute_inject(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("app.ts")).unwrap();
  assert!(content.contains("import cors from 'cors';"));
}

#[test]
fn replace_uses_template_var_in_replacement() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.ts")
    .write_str("const PORT = 3000;")
    .unwrap();

  let step = ReplaceStep {
    target: Target::File {
      file: "config.ts".into(),
    },
    find: "3000".into(),
    replace: "{{ port }}".into(),
    if_not_found: IfNotFound::Error,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("port", "4000");
  execute_replace(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("config.ts")).unwrap();
  assert_eq!(content, "const PORT = 4000;");
}

#[test]
fn append_uses_template_var_in_content() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "# added by {{ author }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("author", "anesis");
  execute_append(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  assert!(content.contains("# added by anesis"));
}

#[test]
fn rename_uses_template_vars_in_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("env.example").write_str("data").unwrap();

  let step = RenameStep {
    from: "{{ src }}".into(),
    to: "{{ dest }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("src", "env.example");
  ctx.insert("dest", ".env.local");
  execute_rename(&step, dir.path(), &ctx).unwrap();

  assert!(!dir.path().join("env.example").exists());
  assert!(dir.path().join(".env.local").exists());
}

#[test]
fn move_uses_template_vars_in_paths() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("data").unwrap();

  let step = MoveStep {
    from: "{{ src }}".into(),
    to: "subdir/{{ dest }}".into(),
  };

  let mut ctx = tera::Context::new();
  ctx.insert("src", "file.txt");
  ctx.insert("dest", "moved.txt");
  execute_move(&step, dir.path(), &ctx).unwrap();

  assert!(!dir.path().join("file.txt").exists());
  assert!(dir.path().join("subdir/moved.txt").exists());
}

#[test]
fn create_uses_template_var_in_path() {
  let dir = assert_fs::TempDir::new().unwrap();

  let step = CreateStep {
    path: ".env.{{ env }}".into(),
    content: "ENV={{ env }}".into(),
    if_exists: IfExists::Overwrite,
  };

  let mut ctx = tera::Context::new();
  ctx.insert("env", "test");
  execute_create(&step, dir.path(), &ctx).unwrap();

  let content = std::fs::read_to_string(dir.path().join(".env.test")).unwrap();
  assert_eq!(content, "ENV=test");
}

// ── append: trailing newline handling ────────────────────────────────────────

#[test]
fn append_to_file_with_trailing_newline_no_double_newline() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("file.txt").write_str("line1\n").unwrap();

  let step = AppendStep {
    target: Target::File {
      file: "file.txt".into(),
    },
    content: "line2".into(),
  };

  execute_append(&step, dir.path(), &empty_ctx()).unwrap();

  let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
  let lines: Vec<&str> = content.lines().collect();
  assert_eq!(lines.len(), 2);
  assert_eq!(lines[1], "line2");
}
