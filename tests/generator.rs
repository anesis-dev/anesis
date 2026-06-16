use std::path::PathBuf;

use anesis::templates::{
  TemplateFile,
  generator::{extract_dir_contents, to_camel_case, to_kebab_case, to_pascal_case, to_snake_case},
};
use tera::{Context, Tera};

fn make_context(project_name: &str) -> Context {
  let mut ctx = Context::new();
  ctx.insert("project_name", project_name);
  ctx.insert("project_name_kebab", &to_kebab_case(project_name));
  ctx.insert("project_name_snake", &to_snake_case(project_name));
  ctx
}

// ── case helpers ──────────────────────────────────────────────────────────────

#[test]
fn kebab_case_replaces_underscores_and_spaces() {
  assert_eq!(to_kebab_case("My_Project Name"), "my-project-name");
  assert_eq!(to_kebab_case("hello"), "hello");
  assert_eq!(to_kebab_case("Hello_World"), "hello-world");
}

#[test]
fn snake_case_replaces_hyphens_and_spaces() {
  assert_eq!(to_snake_case("my-project-name"), "my_project_name");
  assert_eq!(to_snake_case("hello"), "hello");
  assert_eq!(to_snake_case("Hello-World"), "hello_world");
}

// ── extract_dir_contents ──────────────────────────────────────────────────────

#[test]
fn renders_tera_file_and_strips_extension() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("README.md.tera"),
    contents: b"# {{ project_name }}".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("my-app")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
  assert_eq!(content, "# my-app");
}

#[test]
fn copies_non_tera_file_unchanged() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/index.ts"),
    contents: b"console.log('hello')".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("my-app")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("src").join("index.ts")).unwrap();
  assert_eq!(content, "console.log('hello')");
}

#[test]
fn template_vars_kebab_and_snake() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("out.txt.tera"),
    contents: b"{{ project_name_kebab }} {{ project_name_snake }}".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("My_Project")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("out.txt")).unwrap();
  assert_eq!(content, "my-project my_project");
}

#[test]
fn creates_nested_output_directories() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/components/Button.tsx"),
    contents: b"export default () => null".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("app")).unwrap();

  assert!(dir.path().join("src/components/Button.tsx").exists());
}

// ── to_pascal_case ────────────────────────────────────────────────────────────

#[test]
fn pascal_case_from_kebab() {
  assert_eq!(to_pascal_case("my-project-name"), "MyProjectName");
}

#[test]
fn pascal_case_from_snake() {
  assert_eq!(to_pascal_case("my_project_name"), "MyProjectName");
}

#[test]
fn pascal_case_single_word() {
  assert_eq!(to_pascal_case("hello"), "Hello");
}

#[test]
fn pascal_case_already_pascal() {
  assert_eq!(to_pascal_case("MyProject"), "MyProject");
}

#[test]
fn pascal_case_empty_string() {
  assert_eq!(to_pascal_case(""), "");
}

#[test]
fn pascal_case_with_spaces() {
  assert_eq!(to_pascal_case("my project name"), "MyProjectName");
}

// ── to_camel_case ─────────────────────────────────────────────────────────────

#[test]
fn camel_case_from_kebab() {
  assert_eq!(to_camel_case("my-project-name"), "myProjectName");
}

#[test]
fn camel_case_from_snake() {
  assert_eq!(to_camel_case("my_project_name"), "myProjectName");
}

#[test]
fn camel_case_single_word() {
  assert_eq!(to_camel_case("hello"), "hello");
  assert_eq!(to_camel_case("Hello"), "hello");
}

#[test]
fn camel_case_empty_string() {
  assert_eq!(to_camel_case(""), "");
}

#[test]
fn camel_case_with_spaces() {
  assert_eq!(to_camel_case("my project"), "myProject");
}

// ── path traversal ────────────────────────────────────────────────────────────

#[test]
fn path_traversal_blocked_by_extract_dir_contents() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("../../etc/passwd"),
    contents: b"evil".to_vec(),
  }];

  let mut tera = Tera::default();
  let result = extract_dir_contents(&files, dir.path(), &mut tera, &make_context("app"));
  assert!(result.is_err(), "path traversal should be blocked");
}

#[test]
fn path_traversal_with_tera_file_blocked() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("../sibling.txt"),
    contents: b"escaped".to_vec(),
  }];

  let mut tera = Tera::default();
  let result = extract_dir_contents(&files, dir.path(), &mut tera, &make_context("app"));
  assert!(
    result.is_err(),
    "single-level traversal should also be blocked"
  );
}

// ── extract_dir_contents: Tera variable substitution ─────────────────────────

#[test]
fn renders_all_three_case_variables() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("vars.txt.tera"),
    contents: b"{{ project_name }} {{ project_name_kebab }} {{ project_name_snake }}".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("My_App")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("vars.txt")).unwrap();
  assert_eq!(content, "My_App my-app my_app");
}

#[test]
fn multiple_tera_files_rendered_independently() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![
    TemplateFile {
      path: PathBuf::from("a.txt.tera"),
      contents: b"A: {{ project_name }}".to_vec(),
    },
    TemplateFile {
      path: PathBuf::from("b.txt.tera"),
      contents: b"B: {{ project_name_kebab }}".to_vec(),
    },
  ];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("MyApp")).unwrap();

  let a = std::fs::read_to_string(dir.path().join("a.txt")).unwrap();
  let b = std::fs::read_to_string(dir.path().join("b.txt")).unwrap();
  assert_eq!(a, "A: MyApp");
  assert_eq!(b, "B: myapp");
}
