mod common;

use anesis_cli::upgrade::render_upgrade_notice;
use chrono::{TimeZone, Utc};
use common::{
  asset_filename_for_tests, is_cache_fresh_for_tests, is_newer_version_for_tests,
  normalize_version_tag_for_tests, parse_version_for_tests, release_asset_url_for_tests,
};

#[test]
fn normalize_version_tag_strips_leading_v() {
  let version = normalize_version_tag_for_tests("v1.2.3").unwrap();
  assert_eq!(version, "1.2.3");
}

#[test]
fn parse_version_rejects_invalid_formats() {
  let error = parse_version_for_tests("1.2").unwrap_err();
  assert!(error.to_string().contains("patch"));
}

#[test]
fn is_newer_version_compares_numeric_components() {
  assert!(is_newer_version_for_tests("0.7.4", "0.8.0").unwrap());
  assert!(!is_newer_version_for_tests("0.8.0", "0.7.4").unwrap());
}

#[test]
fn asset_filename_appends_exe_for_windows_builds() {
  assert_eq!(
    asset_filename_for_tests("windows-x86_64"),
    "anesis-windows-x86_64.exe"
  );
  assert_eq!(
    asset_filename_for_tests("linux-x86_64"),
    "anesis-linux-x86_64"
  );
}

#[test]
fn release_asset_url_uses_expected_github_pattern() {
  let asset_url = release_asset_url_for_tests("1.2.3", "linux-x86_64");
  assert_eq!(
    asset_url,
    "https://github.com/anesis-dev/anesis/releases/download/v1.2.3/anesis-linux-x86_64"
  );
}

#[test]
fn cache_is_fresh_for_recent_version_checks() {
  let now = Utc.with_ymd_and_hms(2026, 4, 11, 10, 30, 0).unwrap();
  assert!(is_cache_fresh_for_tests(
    "2026-04-11T10:00:00Z",
    "0.8.0",
    now
  ));
}

#[test]
fn cache_is_stale_after_one_hour() {
  let now = Utc.with_ymd_and_hms(2026, 4, 11, 11, 0, 1).unwrap();
  assert!(!is_cache_fresh_for_tests(
    "2026-04-11T10:00:00Z",
    "0.8.0",
    now
  ));
}

#[test]
fn render_upgrade_notice_mentions_upgrade_command() {
  let mut parts = env!("CARGO_PKG_VERSION").split('.');
  let major = parts.next().unwrap();
  let minor = parts.next().unwrap();
  let patch: u64 = parts.next().unwrap().parse().unwrap();
  let latest = format!("{major}.{minor}.{}", patch + 1);
  let notice = render_upgrade_notice(&latest);
  assert!(notice.contains("Run `anesis upgrade` to update."));
  assert!(notice.contains(&format!("v{} → v{}", env!("CARGO_PKG_VERSION"), latest)));
}

#[test]
fn parse_version_succeeds_for_valid_semver() {
  let result = parse_version_for_tests("1.2.3").unwrap();
  assert_eq!(result, (1, 2, 3));
}

#[test]
fn parse_version_succeeds_for_zero_components() {
  let result = parse_version_for_tests("0.0.0").unwrap();
  assert_eq!(result, (0, 0, 0));
}

#[test]
fn parse_version_rejects_too_many_components() {
  assert!(parse_version_for_tests("1.2.3.4").is_err());
}

#[test]
fn parse_version_rejects_non_numeric() {
  assert!(parse_version_for_tests("1.2.beta").is_err());
}

#[test]
fn normalize_version_tag_without_v_prefix_is_unchanged() {
  let version = normalize_version_tag_for_tests("2.5.1").unwrap();
  assert_eq!(version, "2.5.1");
}

#[test]
fn normalize_version_tag_rejects_invalid_semver() {
  assert!(normalize_version_tag_for_tests("v1.2").is_err());
}

#[test]
fn is_newer_version_false_when_equal() {
  assert!(!is_newer_version_for_tests("1.0.0", "1.0.0").unwrap());
}

#[test]
fn is_newer_version_patch_bump() {
  assert!(is_newer_version_for_tests("1.0.0", "1.0.1").unwrap());
}

#[test]
fn is_newer_version_major_bump() {
  assert!(is_newer_version_for_tests("0.9.9", "1.0.0").unwrap());
}

#[test]
fn asset_filename_for_macos_has_no_extension() {
  assert_eq!(
    asset_filename_for_tests("macos-aarch64"),
    "anesis-macos-aarch64"
  );
}

#[test]
fn cache_is_stale_for_invalid_date_format() {
  assert!(!is_cache_fresh_for_tests(
    "not-a-date",
    "0.8.0",
    chrono::Utc::now()
  ));
}
