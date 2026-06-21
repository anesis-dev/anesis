
use clap::Subcommand;

use crate::completions::CompletionShell;

#[derive(Subcommand)]
pub enum AddonCommands {
  #[command(alias = "i", about = "Install and cache an addon (anesis addon i)")]
  Install { addon_id: String },

  #[command(alias = "l", about = "List installed addons (anesis addon l)")]
  List,

  #[command(alias = "r", about = "Remove a cached addon (anesis addon r)")]
  Remove { addon_id: String },

  #[command(
    alias = "p",
    about = "Publish a GitHub repository as an Anesis addon (anesis addon p)"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    alias = "u",
    about = "Update a GitHub repository as an Anesis addon (anesis addon u)"
  )]
  Update {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },

}

#[derive(Subcommand)]
pub enum TemplateCommands {
  #[command(
    alias = "i",
    about = "Download and cache a template locally (anesis template i)"
  )]
  Install {
    #[arg(help = "Name of the template to install")]
    template_name: String,
  },

  #[command(
    alias = "l",
    about = "List all locally installed templates (anesis template l)"
  )]
  List,

  #[command(
    alias = "r",
    about = "Remove an installed template from the local cache (anesis template r)"
  )]
  Remove {
    #[arg(help = "Name of the template to remove")]
    template_name: String,
  },

  #[command(
    alias = "p",
    about = "Publish a GitHub repository as an Anesis template (anesis template p)"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    alias = "u",
    about = "Update a GitHub repository as an Anesis template (anesis template u)"
  )]
  Update {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },

}

#[derive(Subcommand)]
pub enum UseCommands {
  #[command(external_subcommand)]
  External(Vec<String>),
}

#[derive(Subcommand)]
pub enum Commands {
  #[command(alias = "n", about = "Create a new project from a template (anesis n)")]
  New {
    #[arg(help = "Name of the project directory to create")]
    name: String,

    #[arg(help = "Name of the template to use (e.g. react-vite-ts)")]
    template_name: String,
  },

  #[command(alias = "t", about = "Manage templates (anesis t)")]
  Template {
    #[command(subcommand)]
    command: TemplateCommands,
  },

  #[command(alias = "in", about = "Log in to your Anesis account (anesis in)")]
  Login,

  #[command(alias = "out", about = "Log out of your Anesis account (anesis out)")]
  Logout,

  #[command(about = "Show information about the currently logged-in account")]
  Account,

  #[command(alias = "a", about = "Manage addons (anesis a)")]
  Addon {
    #[command(subcommand)]
    command: AddonCommands,
  },

  #[command(
    about = "Run an installed addon command",
    override_usage = "anesis use <ADDON_ID> <COMMAND>",
    arg_required_else_help = true
  )]
  Use {
    #[command(subcommand)]
    command: UseCommands,
  },

  #[command(about = "Download and install the latest Anesis release")]
  Upgrade,

  #[command(about = "Install shell tab completion for anesis")]
  Completions {
    #[arg(value_enum, help = "Shell to install completions for")]
    shell: CompletionShell,
  },
}
