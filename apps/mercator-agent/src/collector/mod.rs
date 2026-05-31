mod battery;
mod network;
mod system;
mod user;

use time::OffsetDateTime;

use crate::config::AgentConfig;
use crate::types::{BatteryInfo, EnrollmentRequest, HeartbeatRequest, NetworkInfo, SystemInfo};

pub fn collect_enroll_payload(agent_version: &str) -> EnrollmentRequest {
    let system = collect_system_info();
    EnrollmentRequest {
        hostname: system::hostname(),
        serial_number: system.serial_number.clone(),
        manufacturer: system.manufacturer.clone(),
        model: system.model.clone(),
        os_name: system.os_name.clone(),
        os_version: system.os_version.clone(),
        agent_version: agent_version.to_string(),
        collected_at: OffsetDateTime::now_utc(),
    }
}

pub fn collect_heartbeat_payload(config: &AgentConfig) -> HeartbeatRequest {
    let system = collect_system_info();
    HeartbeatRequest {
        device_id: config.device_id.clone().unwrap_or_default(),
        hostname: system::hostname(),
        current_user: collect_current_user(),
        agent_version: config.agent_version.clone(),
        occurred_at: OffsetDateTime::now_utc(),
        system,
        network: collect_network_info(),
        battery: collect_battery_info(),
    }
}

pub fn collect_system_info() -> SystemInfo {
    system::collect_system_info()
}

pub fn collect_network_info() -> NetworkInfo {
    network::collect_network_info()
}

pub fn collect_current_user() -> Option<String> {
    user::collect_current_user()
}

pub fn collect_battery_info() -> BatteryInfo {
    battery::collect_battery_info()
}
