//! `anesis account` implementation — fetches and displays user info.
//!
//! Reads the locally stored JWT, sends it to `GET /user/info`, and prints
//! the GitHub login name.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user};

/// Minimal user record returned by `GET /user/info`.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseUser {
  id: String,
  login: String,
  github_id: i32,
  avatar_url: String,
}

/// Fetches the current user's account info and prints their GitHub login.
///
/// Requires the user to be logged in; returns an error otherwise.
pub async fn print_user_info(ctx: &AppContext) -> Result<()> {
  let user = get_user_info(ctx).await?;

  println!("You are registered as @{}", user.login);

  Ok(())
}

/// Fetches `GET /user/info` and returns the parsed [`ResponseUser`].
///
/// Attaches the stored JWT as a `Bearer` token.
pub async fn get_user_info(ctx: &AppContext) -> Result<ResponseUser> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res = ctx
    .client
    .get(format!("{}/user/info", ctx.backend_url))
    .header("Authorization", format!("Bearer {}", user.token))
    .send()
    .await?
    .error_for_status()?;

  let user: ResponseUser = res.json().await?;

  Ok(user)
}
