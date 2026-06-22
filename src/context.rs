use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::paths::AnesisPaths;

pub type CleanupState = Arc<Mutex<Option<PathBuf>>>;

pub struct AppContext {
  pub paths: AnesisPaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
  pub backend_url: String,
  pub frontend_url: String,
}

impl AppContext {
  pub fn new(paths: AnesisPaths, client: Client, cleanup_state: CleanupState) -> Self {
    // let backend_url = "http://localhost:4000".to_string();
    // let frontend_url = "http://localhost:3000".to_string();
    let backend_url = "https://anesis-server.onrender.com".to_string();
    let frontend_url = "https://anesis-dev.vercel.app".to_string();
    Self {
      paths,
      client,
      cleanup_state,
      backend_url,
      frontend_url,
    }
  }
}
