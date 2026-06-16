//! CLI structure and parsing via `clap`.
//!
//! Defines the top-level [`Cli`] struct that clap derives a parser for, and
//! re-exports the [`Commands`] enum from the `commands` sub-module.
//!
//! [`parse`] is the public entry point called from `main.rs`.
//! [`command`] returns the raw `clap::Command` tree, which is also used by
//! the completions module to generate shell-specific completion scripts.

use clap::{CommandFactory, Parser};

pub mod commands;
use commands::Commands;

/// Root CLI struct; clap reads the `#[command(...)]` attribute for the binary
/// name and automatically appends a `--version` flag from `Cargo.toml`.
#[derive(Parser)]
#[command(name = "anesis", version)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}

/// Parses arguments from `std::env::args_os` and exits with an error message
/// on invalid input (clap handles the formatting).
pub fn parse() -> Cli {
  Cli::parse()
}

/// Returns the raw clap `Command` definition without parsing any arguments.
///
/// Used by the completions module to generate shell completion scripts and
/// inject dynamic candidates for installed templates/addons.
pub fn command() -> clap::Command {
  <Cli as CommandFactory>::command()
}
