use clap::{CommandFactory, Parser};

pub mod commands;
use commands::Commands;

#[derive(Parser)]
#[command(name = "anesis", version)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}

pub fn parse() -> Cli {
  Cli::parse()
}

pub fn command() -> clap::Command {
  <Cli as CommandFactory>::command()
}
