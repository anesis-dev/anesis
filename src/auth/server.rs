//! One-shot local HTTP server that receives the OAuth callback.
//!
//! During `anesis login`, the browser is redirected to
//! `http://127.0.0.1:8080/callback?token=<JWT>&name=<user>&state=<nonce>`.
//! This module spins up a minimal Axum server to handle that single request,
//! validates the CSRF state token, extracts the credentials, and shuts the
//! server down.
//!
//! The concurrency model:
//! - The server task and the timeout run concurrently via `tokio::select!`.
//! - Credentials are passed back to the caller through a `oneshot` channel.
//! - A `Notify` signals graceful shutdown so Axum stops accepting new connections
//!   as soon as the callback is handled.

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

/// `Arc<Mutex<Option<Sender>>>` — the `Option` lets the callback handler take
/// ownership of the sender exactly once (subsequent requests get `None`).
type SharedTx = Arc<Mutex<Option<oneshot::Sender<User>>>>;
/// Axum shared state: (token sender, expected CSRF state, frontend URL).
type AppState = (SharedTx, String, String);

/// Credentials received from the OAuth callback.
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
  /// JWT issued by the backend.
  pub token: String,
  /// GitHub username of the authenticated user.
  pub name: String,
}

/// Starts a one-shot local HTTP server on 127.0.0.1:8080 that waits for the
/// OAuth callback redirect.  `expected_state` is the CSRF nonce generated
/// by the caller; the callback validates it before accepting credentials.
pub async fn run_local_auth_server(expected_state: String, frontend_url: &str) -> Result<User> {
  let notify = Arc::new(Notify::new());
  let notify_clone = notify.clone();
  // `oneshot` is used because we only ever expect exactly one callback request.
  let (tx, rx) = oneshot::channel::<User>();

  // Wrap `tx` in Arc<Mutex<Option<_>>> so the Axum handler (which runs in a
  // separate async task) can take the sender without unsafe code.
  let shared_tx: SharedTx = Arc::new(Mutex::new(Some(tx)));
  let state: AppState = (shared_tx, expected_state, frontend_url.to_string());

  let app = Router::new()
    .route("/callback", get(callback))
    .with_state(state);

  let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

  // Graceful shutdown: the server task will exit once `notify` is signalled.
  let server = axum::serve(listener, app).with_graceful_shutdown(async move {
    notify_clone.notified().await;
  });

  // Race three futures: server completion, callback result, and timeout.
  // The first branch to complete wins; the others are dropped.
  tokio::select! {
    result = server => {
      result?;
      Err(anyhow!("Server stopped unexpectedly"))
    }
    user = rx => {
      // Callback was received and credentials sent — signal server shutdown.
      notify.notify_one();
      Ok(user?)
    }
    _ = tokio::time::sleep(Duration::from_secs(600)) => {
      notify.notify_one();
      Err(anyhow!("Login timed out after 10 minutes. Please try again."))
    }
  }
}

/// Axum handler for `GET /callback`.
///
/// Validates the CSRF `state` parameter, extracts `token` and `name`, then
/// sends the `User` through the oneshot channel and redirects the browser to
/// the success page.
async fn callback(
  State((shared_tx, expected_state, frontend_url)): State<AppState>,
  Query(params): Query<HashMap<String, String>>,
) -> Redirect {
  // Validate CSRF state token.  The backend must forward the `?state=`
  // query param it received at /auth/cli-login through to this redirect.
  match params.get("state") {
    Some(state) if state == &expected_state => {}
    Some(_) => return Redirect::to(&format!("{}/cli/error?reason=invalid_state", frontend_url)),
    None => return Redirect::to(&format!("{}/cli/error?reason=missing_state", frontend_url)),
  }

  if let Some(token) = params.get("token")
    && let Some(user_name) = params.get("name")
  {
    let mut guard = shared_tx.lock().await;

    // `guard.take()` consumes the sender so a second callback request is a no-op.
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
