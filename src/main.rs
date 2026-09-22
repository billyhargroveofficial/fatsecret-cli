use std::io::IsTerminal;

use anyhow::Context;
use clap::{CommandFactory, Parser};
use fatsecret_cli::{
    cli::{Cli, ColorChoice, Commands},
    commands,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        let level = match cli.verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        tracing_subscriber::EnvFilter::new(level)
    };
    let ansi = match cli.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stderr().is_terminal(),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(ansi)
        .with_writer(std::io::stderr)
        .init();

    // Completions need no profile; handle before dispatch.
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        clap_complete::generate(*shell, &mut cmd, "fatsecret-cli", &mut std::io::stdout());
        return Ok(());
    }

    commands::dispatch(cli)
        .await
        .context("fatsecret-cli failed")?;
    Ok(())
}
