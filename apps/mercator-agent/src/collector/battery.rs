use crate::types::BatteryInfo;

pub fn collect_battery_info() -> BatteryInfo {
    BatteryInfo {
        percent: None,
        is_charging: None,
    }
}
