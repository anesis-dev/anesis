// Re-exports of test utilities from src modules
// This centralizes all _for_tests functions in one place.
//
// Each integration test binary pulls in this module via `mod common;` but only
// uses the subset of helpers it needs, so the unused re-exports are expected.
#![allow(unused_imports)]

// Auth utilities
pub use anesis::auth::login::{generate_state_token_for_tests, write_auth_file_for_tests};

// Archive utilities
pub use anesis::utils::archive::strip_archive_path_for_tests;

// Cleanup utilities
pub use anesis::utils::cleanup::cleanup_incomplete_template_for_tests;

// Template utilities
pub use anesis::templates::install::classify_install_state_for_tests as template_classify_install_state_for_tests;

// Addon utilities
pub use anesis::addons::install::classify_install_state_for_tests as addon_classify_install_state_for_tests;
pub use anesis::addons::runner::{
  rerun_prompt_message_for_tests, should_fallback_to_cached_manifest_for_tests,
};

// Upgrade utilities
pub use anesis::upgrade::{
  asset_filename_for_tests, is_cache_fresh_for_tests, is_newer_version_for_tests,
  normalize_version_tag_for_tests, parse_version_for_tests, release_asset_url_for_tests,
};
