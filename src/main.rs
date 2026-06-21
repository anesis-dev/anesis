use anesis::{
  addons,
  auth::{account::print_user_info, login::login, logout::logout},
  cache::{get_installed_templates, remove_template_from_cache},
  cli::{
    self,
    commands::{AddonCommands, Commands, TemplateCommands, UseCommands},
  },
  completions, config,
  context::AppContext,
  templates::{
    generator::extract_template,
    install::{InstallResult, install_template, record_template_use},
    loader::get_files,
    publish::publish,
    update::update,
  },
  upgrade::{check_cli_version_cached, render_upgrade_notice, upgrade_cli},
  utils::{
    errors::print_error,
    ui::spinner,
    validate::{is_valid_github_repo_url, validate_project_name, validate_template_name},
  },
};
use anyhow::{Context, Result};

#[tokio::main]
async fn main() {
  config::init_env();
  completions::complete_env();
  if let Err(err) = run().await {
    print_error(&err);
    std::process::exit(1);
  }
}

async fn run() -> Result<()> {
  let cli = cli::parse();
  let ctx = config::build_app_context()?;

  let skip_version_notice = matches!(
    &cli.command,
    Commands::Upgrade | Commands::Completions { .. }
  );
  let version_check_handle = if skip_version_notice {
    None
  } else {
    let client = ctx.client.clone();
    let version_check_path = ctx.paths.version_check.clone();
    Some(tokio::spawn(async move {
      check_cli_version_cached(&client, &version_check_path).await
    }))
  };

  match cli.command {
    Commands::New {
      name,
      template_name,
    } => {
      validate_project_name(&name)?;
      create_new_project(&ctx, &name, &template_name).await?;
    }
    Commands::Template { command } => match command {
      TemplateCommands::Install { template_name } => {
        validate_template_name(&template_name)?;
        let install_result = install_template(&ctx, &template_name).await?;
        match install_result {
          InstallResult::UpToDate => {
            println!("{}", InstallResult::up_to_date_message(&template_name));
          }
          _ => {
            if let Some(message) = install_result.message(&template_name) {
              println!("{message}");
            }
          }
        }
      }
      TemplateCommands::List => {
        get_installed_templates(&ctx.paths.templates)?;
      }
      TemplateCommands::Remove { template_name } => {
        remove_template_from_cache(&ctx.paths.templates, &template_name)?;
      }
      TemplateCommands::Publish {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&template_url)?;
        publish(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
      TemplateCommands::Update {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        update(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
    },
    Commands::Login => {
      login(&ctx.paths.auth, &ctx.backend_url, &ctx.frontend_url).await?;
    }
    Commands::Logout => {
      logout(&ctx.paths.auth)?;
    }
    Commands::Account => {
      print_user_info(&ctx).await?;
    }
    Commands::Addon { command } => match command {
      AddonCommands::Install { addon_id } => {
        let install_result = addons::install::install_addon(&ctx, &addon_id).await?;
        match &install_result {
          addons::install::AddonInstallResult::UpToDate(_) => {
            println!(
              "{}",
              addons::install::AddonInstallResult::up_to_date_message(&addon_id)
            );
          }
          _ => {
            if let Some(message) = install_result.message(&addon_id) {
              println!("{message}");
            }
          }
        }
      }
      AddonCommands::List => {
        addons::cache::get_installed_addons(&ctx.paths.addons)?;
      }
      AddonCommands::Remove { addon_id } => {
        addons::cache::remove_addon_from_cache(&ctx.paths.addons, &addon_id)?;
      }
      AddonCommands::Publish {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::publish::publish_addon(&ctx, &addon_url, visibility, credential_id, org_id).await?;
      }
      AddonCommands::Update {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::update::update_addon(&ctx, &addon_url, visibility, credential_id, org_id).await?;
      }
    },
    Commands::Use { command } => match command {
      UseCommands::External(args) => {
        let addon_id = args
          .first()
          .context("Usage: anesis use <addon-id> <command>")?;
        let command_name = args
          .get(1)
          .context("Usage: anesis use <addon-id> <command>")?;
        let project_root = std::env::current_dir()?;
        addons::runner::run_addon_command(&ctx, addon_id, command_name, &project_root).await?;
      }
    },
    Commands::Upgrade => {
      upgrade_cli(&ctx).await?;
    }
    Commands::Completions { shell } => {
      completions::install_completions(shell)?;
    }
  }

  if let Some(version_check_handle) = version_check_handle
    && let Ok(Ok(Some(latest_version))) = version_check_handle.await
  {
    println!("{}", render_upgrade_notice(&latest_version));
  }

  Ok(())
}

async fn create_new_project(
  ctx: &AppContext,
  project_name: &str,
  template_name: &str,
) -> Result<()> {
  let files = get_files(ctx, template_name).await?;
  if project_name != "." {
    println!("Generating project '{project_name}' from template '{template_name}'...");
  } else {
    println!("Generating project from template '{template_name}'...");
  }

  extract_template(&files, project_name)?;

  let sp = spinner("Finishing up...");
  record_template_use(ctx, template_name).await;
  sp.finish_and_clear();

  if project_name != "." {
    println!("✅ Project '{project_name}' created successfully!");
    println!("\nNext steps:");
    println!("  cd {}", project_name);
  } else {
    println!("✅ Project created successfully!");
  }
  Ok(())
}
