use std::sync::{Mutex, OnceLock};

use anesis::auth::server::run_local_auth_server;

// Each test in this file starts a real Axum server on 127.0.0.1:8080.
// The global mutex ensures no two tests bind the same port simultaneously.
static PORT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn acquire_port() -> std::sync::MutexGuard<'static, ()> {
  PORT_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[tokio::test]
async fn callback_with_valid_state_returns_user() {
  let _lock = acquire_port();
  let state = "validstate0000000000000000000000".to_string();
  let state_clone = state.clone();

  let server_handle =
    tokio::spawn(async move { run_local_auth_server(state_clone, "http://localhost:3000").await });

  // Give the server time to bind before sending the request.
  tokio::time::sleep(std::time::Duration::from_millis(200)).await;

  let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .unwrap();

  client
    .get("http://127.0.0.1:8080/callback")
    .query(&[
      ("state", state.as_str()),
      ("token", "secret-token"),
      ("name", "testuser"),
    ])
    .send()
    .await
    .unwrap();

  let user = server_handle.await.unwrap().unwrap();
  assert_eq!(user.token, "secret-token");
  assert_eq!(user.name, "testuser");
}

#[tokio::test]
async fn callback_with_invalid_state_redirects_to_error() {
  let _lock = acquire_port();
  let state = "correctstate00000000000000000000".to_string();

  let server_handle =
    tokio::spawn(async move { run_local_auth_server(state, "http://localhost:3000").await });

  tokio::time::sleep(std::time::Duration::from_millis(200)).await;

  let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .unwrap();

  let res = client
    .get("http://127.0.0.1:8080/callback")
    .query(&[("state", "wrongstate"), ("token", "tok"), ("name", "user")])
    .send()
    .await
    .unwrap();

  let location = res
    .headers()
    .get("location")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");
  assert!(
    location.contains("invalid_state"),
    "expected invalid_state redirect, got: {location}"
  );

  // Server is still running (it didn't accept the request). Abort it.
  server_handle.abort();
}

#[tokio::test]
async fn callback_without_state_redirects_to_error() {
  let _lock = acquire_port();
  let server_handle = tokio::spawn(async move {
    run_local_auth_server("somestate".to_string(), "http://localhost:3000").await
  });

  tokio::time::sleep(std::time::Duration::from_millis(200)).await;

  let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .build()
    .unwrap();

  let res = client
    .get("http://127.0.0.1:8080/callback")
    .query(&[("token", "tok"), ("name", "user")])
    .send()
    .await
    .unwrap();

  let location = res
    .headers()
    .get("location")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");
  assert!(
    location.contains("missing_state"),
    "expected missing_state redirect, got: {location}"
  );

  server_handle.abort();
}
