use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use axum::{
  Router,
  extract::{Query, State},
  response::Redirect,
  routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::{
  sync::{Mutex, Notify, oneshot},
  time::Duration,
};

type SharedTx = Arc<Mutex<Option<oneshot::Sender<User>>>>;
type AppState = (SharedTx, String, String);

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
  pub token: String,
  pub name: String,
}

pub async fn run_local_auth_server(expected_state: String, frontend_url: &str) -> Result<User> {
  let notify = Arc::new(Notify::new());
  let notify_clone = notify.clone();
  let (tx, rx) = oneshot::channel::<User>();

  let shared_tx: SharedTx = Arc::new(Mutex::new(Some(tx)));
  let state: AppState = (shared_tx, expected_state, frontend_url.to_string());

  let app = Router::new()
    .route("/callback", get(callback))
    .with_state(state);

  // Bind with SO_REUSEADDR so a fresh login can rebind the port immediately,
  // even while a connection from a previous login lingers in TIME_WAIT.
  let socket = tokio::net::TcpSocket::new_v4()?;
  socket.set_reuseaddr(true)?;
  socket.bind("127.0.0.1:8080".parse()?)?;
  let listener = socket.listen(1024)?;

  let server = axum::serve(listener, app).with_graceful_shutdown(async move {
    notify_clone.notified().await;
  });

  tokio::select! {
    result = server => {
      result?;
      Err(anyhow!("Server stopped unexpectedly"))
    }
    user = rx => {
      notify.notify_one();
      Ok(user?)
    }
    _ = tokio::time::sleep(Duration::from_secs(600)) => {
      notify.notify_one();
      Err(anyhow!("Login timed out after 10 minutes. Please try again."))
    }
  }
}

async fn callback(
  State((shared_tx, expected_state, frontend_url)): State<AppState>,
  Query(params): Query<HashMap<String, String>>,
) -> Redirect {
  match params.get("state") {
    Some(state) if state == &expected_state => {}
    Some(_) => return Redirect::to(&format!("{}/cli/error?reason=invalid_state", frontend_url)),
    None => return Redirect::to(&format!("{}/cli/error?reason=missing_state", frontend_url)),
  }

  if let Some(token) = params.get("token")
    && let Some(user_name) = params.get("name")
  {
    let mut guard = shared_tx.lock().await;

    if let Some(tx) = guard.take() {
      let _ = tx.send(User {
        name: user_name.to_string(),
        token: token.to_string(),
      });
    }

    Redirect::to(&format!("{}/cli/success", frontend_url))
  } else {
    Redirect::to(&format!("{}/cli/error", frontend_url))
  }
}
