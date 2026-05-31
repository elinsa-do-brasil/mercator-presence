use sysinfo::{Motherboard, Product, System};

use crate::types::SystemInfo;

pub fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".to_string())
}

pub fn collect_system_info() -> SystemInfo {
    let mut system = System::new_all();
    system.refresh_all();

    let motherboard = Motherboard::new();
    let manufacturer = clean_optional(Product::vendor_name())
        .or_else(|| motherboard.as_ref().and_then(Motherboard::vendor_name));
    let model = clean_optional(Product::name())
        .or_else(|| clean_optional(Product::version()))
        .or_else(|| motherboard.as_ref().and_then(Motherboard::name));
    let serial_number = clean_optional(Product::serial_number())
        .or_else(|| motherboard.as_ref().and_then(Motherboard::serial_number));

    SystemInfo {
        manufacturer,
        model,
        serial_number,
        os_name: clean_optional(System::name()),
        os_version: clean_optional(System::long_os_version()).or_else(System::os_version),
        cpu_brand: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|brand| !brand.is_empty()),
        total_memory_bytes: system.total_memory(),
        used_memory_bytes: system.used_memory(),
        uptime_seconds: System::uptime(),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
