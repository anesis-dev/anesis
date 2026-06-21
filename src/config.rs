use crate::{
  context::{AppContext, CleanupState},
  paths::AnesisPaths,
  utils::cleanup::setup_ctrlc_handler,
};
use anyhow::Result;
use reqwest::Client;
use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

pub fn init_env() {
  env_logger::init();
  dotenvy::dotenv().ok();
}

pub fn build_app_context() -> Result<AppContext> {
  let anesis_paths = AnesisPaths::new()?;
  anesis_paths.ensure_directories()?;

  let client = Client::builder()
    .connect_timeout(Duration::from_secs(10))
    .timeout(Duration::from_secs(90))
    .build()?;

  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  setup_ctrlc_handler(cleanup_state.clone(), anesis_paths.templates.clone())?;

  Ok(AppContext::new(anesis_paths, client, cleanup_state))
}
