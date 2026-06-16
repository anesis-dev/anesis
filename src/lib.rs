//! Root library crate for the Anesis CLI.
//!
//! Re-exports all public modules and defines the two foundational types that
//! are threaded through every command handler: [`AppContext`] and
//! [`CleanupState`].

pub mod addons;
pub mod auth;
pub mod cache;
pub mod cli;
pub mod completions;
pub mod paths;
pub mod templates;
pub mod upgrade;
pub mod utils;

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::paths::AnesisPaths;

/// Shared mutable state that tracks which directory is currently being
/// downloaded/extracted.  Written before a download starts and cleared after
/// it completes so the Ctrl+C handler knows what to remove on abort.
///
/// `Arc<Mutex<_>>` is used because the cleanup handler runs on a separate OS
/// thread injected by the `ctrlc` crate, while the main async task also needs
/// to write the path.
pub type CleanupState = Arc<Mutex<Option<PathBuf>>>;

/// Application-wide context passed by reference into every command handler.
///
/// Centralising these resources avoids threading dozens of individual
/// parameters through the call stack and makes it easy to swap
/// implementations in tests.
pub struct AppContext {
  /// Resolved paths under `~/.anesis/`.
  pub paths: AnesisPaths,
  /// Shared HTTP client — reusing a single client is important for
  /// connection pool reuse and consistent timeout configuration.
  pub client: Client,
  /// Path of the directory currently being extracted; `None` when idle.
  pub cleanup_state: CleanupState,
  /// Base URL of the Anesis backend (e.g. `https://anesis-server.onrender.com`).
  pub backend_url: String,
  /// Base URL of the Anesis web frontend (used for OAuth redirect targets).
  pub frontend_url: String,
}

impl AppContext {
  /// Constructs the context, resolving backend/frontend URLs from environment
  /// variables with sensible localhost defaults for local development.
  pub fn new(paths: AnesisPaths, client: Client, cleanup_state: CleanupState) -> Self {
    // let backend_url =
    //   std::env::var("ANESIS_BACKEND_URL").unwrap_or_else(|_| "http://localhost:4000".to_string());
    // let frontend_url =
    //   std::env::var("ANESIS_FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let backend_url = std::env::var("ANESIS_BACKEND_URL")
      .unwrap_or_else(|_| "https://anesis-server.onrender.com".to_string());
    let frontend_url = std::env::var("ANESIS_FRONTEND_URL")
      .unwrap_or_else(|_| "https://anesis-dev.vercel.app".to_string());
    Self {
      paths,
      client,
      cleanup_state,
      backend_url,
      frontend_url,
    }
  }
}
