use anesis_cli::utils::validate::{
  is_valid_github_repo_url, validate_project_name, validate_template_name,
};

// ── validate_project_name ─────────────────────────────────────────────────────

#[test]
fn project_name_dot_is_valid() {
  assert!(validate_project_name(".").is_ok());
}

#[test]
fn project_name_normal() {
  assert!(validate_project_name("my-project").is_ok());
  assert!(validate_project_name("MyProject123").is_ok());
  assert!(validate_project_name("my_project.v2").is_ok());
}

#[test]
fn project_name_empty_is_err() {
  assert!(validate_project_name("").is_err());
}

#[test]
fn project_name_too_long_is_err() {
  let long = "a".repeat(256);
  assert!(validate_project_name(&long).is_err());
}

#[test]
fn project_name_invalid_chars() {
  for name in ["my project", "my/project", "my@project", "my!project"] {
    assert!(
      validate_project_name(name).is_err(),
      "{name} should be invalid"
    );
  }
}

#[test]
fn project_name_starts_with_dot() {
  assert!(validate_project_name(".hidden").is_err());
}

#[test]
fn project_name_ends_with_dot() {
  assert!(validate_project_name("project.").is_err());
}

#[test]
fn project_name_ends_with_space() {
  assert!(validate_project_name("project ").is_err());
}

#[test]
fn project_name_reserved_windows() {
  for name in ["CON", "con", "NUL", "nul", "COM1", "LPT9"] {
    assert!(
      validate_project_name(name).is_err(),
      "{name} should be reserved"
    );
  }
}

// ── is_valid_github_repo_url ──────────────────────────────────────────────────

#[test]
fn github_url_valid() {
  assert!(is_valid_github_repo_url("https://github.com/owner/repo").is_ok());
  assert!(is_valid_github_repo_url("https://github.com/anesis-dev/anesis").is_ok());
}

#[test]
fn github_url_not_github_domain() {
  assert!(is_valid_github_repo_url("https://gitlab.com/owner/repo").is_err());
  assert!(is_valid_github_repo_url("https://example.com/owner/repo").is_err());
}

#[test]
fn github_url_invalid_format() {
  assert!(is_valid_github_repo_url("not-a-url").is_err());
  assert!(is_valid_github_repo_url("").is_err());
}

#[test]
fn github_url_no_repo_path() {
  assert!(is_valid_github_repo_url("https://github.com/owner").is_err());
  assert!(is_valid_github_repo_url("https://github.com/").is_err());
}

// ── validate_template_name ────────────────────────────────────────────────────

#[test]
fn template_name_valid() {
  for name in ["react-vite-ts", "NextJS", "my_template", "template123"] {
    assert!(
      validate_template_name(name).is_ok(),
      "{name} should be valid"
    );
  }
}

#[test]
fn template_name_invalid() {
  for name in ["my template", "my.template", "my/template", "my@template"] {
    assert!(
      validate_template_name(name).is_err(),
      "{name} should be invalid"
    );
  }
}

#[test]
fn template_name_empty_is_err() {
  assert!(validate_template_name("").is_err());
}
