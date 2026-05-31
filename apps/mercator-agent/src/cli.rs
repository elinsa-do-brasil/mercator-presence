use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mercator-agent")]
#[command(version)]
#[command(about = "Mercator device inventory and presence agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the agent in the foreground.
    Run,
    /// Windows Service entry point used by the Service Control Manager.
    #[command(name = "run-service", hide = true)]
    RunService,
    /// Install, remove, start, or stop the Windows Service.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    /// Enroll this device with Mercator using a one-time enrollment token.
    Enroll {
        #[arg(long)]
        server_url: String,
        #[arg(long)]
        token: String,
    },
    /// Collect inventory and send one heartbeat.
    HeartbeatOnce,
    /// Inspect local agent configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ServiceCommand {
    /// Install the Windows Service.
    Install,
    /// Remove the Windows Service without deleting local config.
    Uninstall,
    /// Start the Windows Service.
    Start,
    /// Stop the Windows Service.
    Stop,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the current config with secrets masked.
    Show,
}
