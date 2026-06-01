//! Olá, patinho! Este é o módulo `collector`.
//! Pense nele como o nosso "painel de sensores" do computador!
//! O seu trabalho é organizar todas as subpastas e arquivos de coleta física da máquina
//! (sistema, bateria, rede e usuário logado) e expor funções centralizadas e limpas para o resto do app.

mod battery;
mod network;
mod system;
mod user;

use crate::types::{BatteryInfo, NetworkInfo};

/// Patinho, esta função centraliza a coleta de informações gerais do hardware e SO da máquina.
/// Ela retorna um snapshot contendo marca, modelo, número de série, CPU, RAM e uptime!
pub fn collect_system_info() -> system::SystemSnapshot {
    system::collect_system_info()
}

/// Coleta dados em tempo real sobre a rede em que a máquina está conectada (WiFi/cabo, IPs, MACs, etc.).
pub fn collect_network_info() -> NetworkInfo {
    network::collect_network_info()
}

/// Coleta o nome do usuário logado na máquina neste exato momento.
/// P.S.: Patinho, esta função está reservada para uso futuro no contrato da API!
#[allow(dead_code)]
pub fn collect_current_user() -> Option<String> {
    user::collect_current_user()
}

/// Coleta o status de energia da bateria física do dispositivo (carga e se está carregando).
pub fn collect_battery_info() -> BatteryInfo {
    battery::collect_battery_info()
}

