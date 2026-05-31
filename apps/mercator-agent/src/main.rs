mod api;
mod app;
mod cli;
mod collector;
mod config;
mod logging;
mod security;
mod service;
mod types;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let _log_guard = logging::init()?;

    match cli.command {
        Command::Run => app::run_foreground().await,
        Command::RunService => service::run_dispatcher(),
        Command::Enroll { server_url, token } => app::enroll(&server_url, &token).await,
        Command::HeartbeatOnce => app::heartbeat_once().await,
        Command::Service { command } => service::handle_command(command),
        Command::Config { command } => app::handle_config(command),
    }
}
