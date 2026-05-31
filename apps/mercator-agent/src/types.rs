use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentRequest {
    pub hostname: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub collected_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentResponse {
    pub device_id: String,
    pub device_token: String,
    pub heartbeat_interval_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub device_id: String,
    pub hostname: String,
    pub current_user: Option<String>,
    pub agent_version: String,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub system: SystemInfo,
    pub network: NetworkInfo,
    pub battery: BatteryInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub cpu_brand: Option<String>,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub network_type: String,
    pub private_ips: Vec<String>,
    pub mac_addresses: Vec<String>,
    pub gateway_ip: Option<String>,
    pub ssid: Option<String>,
    pub bssid_hash: Option<String>,
    pub public_ip_seen_by_server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub percent: Option<u8>,
    pub is_charging: Option<bool>,
}

#[allow(dead_code)]
/// Future message-poll contract. The MVP does not poll or render messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePollResponse {
    pub messages: Vec<AgentMessage>,
}

#[allow(dead_code)]
/// Future HTML message contract for a separate notifier app.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub title: String,
    pub html_body: String,
}

#[allow(dead_code)]
/// Future message acknowledgement contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAckRequest {
    pub message_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub acknowledged_at: OffsetDateTime,
}
