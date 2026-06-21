use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{auth::token::get_auth_user, context::AppContext};

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseUser {
  id: String,
  login: String,
  github_id: i32,
  avatar_url: String,
}

pub async fn print_user_info(ctx: &AppContext) -> Result<()> {
  let user = get_user_info(ctx).await?;

  println!("You are registered as @{}", user.login);

  Ok(())
}

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
