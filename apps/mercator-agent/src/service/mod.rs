use anyhow::Result;

use crate::cli::ServiceCommand;

#[cfg(windows)]
pub const SERVICE_NAME: &str = "MercatorAgent";
#[cfg(windows)]
pub const SERVICE_DISPLAY_NAME: &str = "Mercator Agent";
#[cfg(windows)]
pub const SERVICE_DESCRIPTION: &str = "Mercator device inventory and presence agent";

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub fn handle_command(command: ServiceCommand) -> Result<()> {
    windows::handle_command(command)
}

#[cfg(not(windows))]
pub fn handle_command(_command: ServiceCommand) -> Result<()> {
    anyhow::bail!("Windows Service commands are only available on Windows")
}

#[cfg(windows)]
pub fn run_dispatcher() -> Result<()> {
    windows::run_dispatcher()
}

#[cfg(not(windows))]
pub fn run_dispatcher() -> Result<()> {
    anyhow::bail!("Windows Service dispatcher is only available on Windows")
}
