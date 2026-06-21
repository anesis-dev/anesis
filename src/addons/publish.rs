use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{auth::token::get_auth_user, context::AppContext, utils::ui::spinner};

#[derive(Serialize)]
struct PublishAddonDto {
  url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  visibility: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  repo_credential_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  organization_id: Option<String>,
}

#[derive(Deserialize)]
struct PublishAddonResponse {
  message: String,
  addon_id: String,
}

pub async fn publish_addon(
  ctx: &AppContext,
  addon_url: &str,
  visibility: Option<String>,
  credential_id: Option<String>,
  org_id: Option<String>,
) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let sp = spinner("Publishing addon to registry...");
  let res: PublishAddonResponse = ctx
    .client
    .post(format!("{}/addon/publish", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishAddonDto {
      url: addon_url.to_string(),
      visibility,
      repo_credential_id: credential_id,
      organization_id: org_id,
    })
    .send()
    .await
    .inspect_err(|_| sp.finish_and_clear())?
    .error_for_status()
    .inspect_err(|_| sp.finish_and_clear())?
    .json()
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  println!("✅ {}", res.message);
  println!("   Addon: {}", res.addon_id);
  Ok(())
}
