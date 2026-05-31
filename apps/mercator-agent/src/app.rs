use std::time::Duration;

use anyhow::{Context, Result, bail};
use tracing::{error, info, warn};

use crate::cli::ConfigCommand;
use crate::collector::{collect_enroll_payload, collect_heartbeat_payload};
use crate::config::{self, AgentConfig};
use crate::security::mask_secret;

const BACKOFF_SECONDS: [u64; 5] = [60, 120, 300, 600, 900];

pub async fn run_foreground() -> Result<()> {
    info!("starting mercator-agent in foreground");
    run_loop(shutdown_on_ctrl_c()).await
}

pub async fn run_loop(shutdown: impl Future<Output = ()>) -> Result<()> {
    let config = config::load_config()
        .context("agent is not enrolled; run `mercator-agent enroll --server-url <URL> --token <TOKEN>` first")?;
    config.validate_enrolled()?;

    let mut failures = 0usize;
    send_heartbeat_from_config(&config).await?;

    tokio::pin!(shutdown);
    loop {
        let wait_seconds = if failures == 0 {
            config.heartbeat_interval_seconds
        } else {
            BACKOFF_SECONDS[failures.saturating_sub(1).min(BACKOFF_SECONDS.len() - 1)]
        };

        tokio::select! {
            () = &mut shutdown => {
                info!("shutdown signal received");
                return Ok(());
            }
            () = tokio::time::sleep(Duration::from_secs(wait_seconds)) => {
                match send_heartbeat_from_config(&config).await {
                    Ok(()) => {
                        failures = 0;
                    }
                    Err(error) => {
                        failures = failures.saturating_add(1);
                        warn!(%error, "heartbeat failed; applying backoff");
                    }
                }
            }
        }
    }
}

pub async fn enroll(server_url: &str, enrollment_token: &str) -> Result<()> {
    let normalized_url = config::normalize_server_url(server_url)?;
    let payload = collect_enroll_payload(config::DEFAULT_AGENT_VERSION);
    let response = crate::api::enroll(&normalized_url, enrollment_token, &payload)
        .await
        .context("enrollment request failed")?;
    let config = AgentConfig::new_enrolled(
        normalized_url,
        response.device_id,
        response.device_token,
        response.heartbeat_interval_seconds,
    )?;
    config::save_config(&config)?;

    println!(
        "Enrollment completed. deviceId={} heartbeatIntervalSeconds={}",
        config
            .device_id
            .as_deref()
            .map(mask_secret)
            .unwrap_or_else(|| "****".to_string()),
        config.heartbeat_interval_seconds
    );
    Ok(())
}

pub async fn heartbeat_once() -> Result<()> {
    let config = config::load_config().context("cannot send heartbeat without config")?;
    config.validate_enrolled()?;
    let payload = collect_heartbeat_payload(&config);
    let result = send_heartbeat_payload(&config, &payload).await;

    println!("hostname: {}", payload.hostname);
    println!(
        "serial: {}",
        payload
            .system
            .serial_number
            .as_deref()
            .unwrap_or("unavailable")
    );
    println!(
        "currentUser: {}",
        payload.current_user.as_deref().unwrap_or("unavailable")
    );
    println!("privateIps: {}", payload.network.private_ips.join(", "));

    match result {
        Ok(()) => {
            println!("status: sent");
            Ok(())
        }
        Err(error) => {
            println!("status: failed");
            Err(error)
        }
    }
}

pub fn handle_config(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show => show_config(),
    }
}

fn show_config() -> Result<()> {
    match config::load_config() {
        Ok(config) => {
            let display = config.display();
            println!("{}", serde_json::to_string_pretty(&display)?);
            Ok(())
        }
        Err(config::ConfigError::Missing(path)) => {
            bail!("config file does not exist at {}", path.display())
        }
        Err(error) => Err(error.into()),
    }
}

async fn send_heartbeat_from_config(config: &AgentConfig) -> Result<()> {
    let payload = collect_heartbeat_payload(config);
    send_heartbeat_payload(config, &payload).await
}

async fn send_heartbeat_payload(
    config: &AgentConfig,
    payload: &crate::types::HeartbeatRequest,
) -> Result<()> {
    let device_token = config
        .device_token
        .as_deref()
        .context("deviceToken is missing from config")?;
    crate::api::send_heartbeat(&config.server_url, device_token, payload)
        .await
        .context("heartbeat request failed")?;
    info!(
        hostname = %payload.hostname,
        private_ip_count = payload.network.private_ips.len(),
        "heartbeat sent"
    );
    Ok(())
}

async fn shutdown_on_ctrl_c() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to listen for Ctrl+C");
    }
}
