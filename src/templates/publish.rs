use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user, utils::ui::spinner};

#[derive(Deserialize, Serialize)]
pub struct PublishTemplateDto {
  pub url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub visibility: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub repo_credential_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub organization_id: Option<String>,
}

#[derive(Deserialize)]
struct PublishTemplateResponse {
  message: String,
  name: String,
}

pub async fn publish(
  ctx: &AppContext,
  template_url: &str,
  visibility: Option<String>,
  credential_id: Option<String>,
  org_id: Option<String>,
) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let sp = spinner("Publishing template to registry...");
  let res: PublishTemplateResponse = ctx
    .client
    .post(format!("{}/template/publish", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishTemplateDto {
      url: template_url.to_string(),
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
  println!("   Template: {}", res.name);
  Ok(())
}
