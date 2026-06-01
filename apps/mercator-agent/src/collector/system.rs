use sysinfo::{Motherboard, Product, System};

use crate::types::{DeviceInfo, SystemInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub device: DeviceInfo,
    pub system: SystemInfo,
}

pub fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".to_string())
}

pub fn collect_system_info() -> SystemSnapshot {
    let mut system = System::new_all();
    system.refresh_all();

    let motherboard = Motherboard::new();
    let manufacturer = clean_optional(Product::vendor_name())
        .or_else(|| motherboard.as_ref().and_then(Motherboard::vendor_name));
    let model = clean_optional(Product::name())
        .or_else(|| clean_optional(Product::version()))
        .or_else(|| motherboard.as_ref().and_then(Motherboard::name));
    let serial_number = clean_serial(Product::serial_number())
        .or_else(|| motherboard.as_ref().and_then(Motherboard::serial_number))
        .and_then(|serial| clean_serial(Some(serial)));
    let os_version = clean_optional(System::os_version())
        .or_else(System::kernel_version)
        .or_else(System::long_os_version);

    SystemSnapshot {
        device: DeviceInfo {
            hostname: hostname(),
            serial_number,
            manufacturer,
            model,
            asset_tag: None,
        },
        system: SystemInfo {
            os_name: clean_optional(System::name()),
            os_build: os_version.as_deref().and_then(os_build_from_version),
            os_version,
            cpu_brand: system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().trim().to_string())
                .filter(|brand| !brand.is_empty()),
            total_memory_bytes: system.total_memory(),
            used_memory_bytes: system.used_memory(),
            uptime_seconds: System::uptime(),
        },
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_serial(value: Option<String>) -> Option<String> {
    clean_optional(value).filter(|value| !is_useless_serial(value))
}

fn is_useless_serial(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "to be filled by o.e.m." | "default string" | "system serial number" | "00000000"
    )
}

fn os_build_from_version(version: &str) -> Option<String> {
    version
        .rsplit('.')
        .next()
        .map(str::trim)
        .filter(|build| !build.is_empty() && build.chars().all(|char| char.is_ascii_digit()))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_serial_should_drop_common_placeholder_values() {
        assert_eq!(
            clean_serial(Some("To be filled by O.E.M.".to_string())),
            None
        );
    }

    #[test]
    fn os_build_from_version_should_use_last_numeric_segment() {
        assert_eq!(
            os_build_from_version("10.0.19045"),
            Some("19045".to_string())
        );
    }
}
