use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// Aperto de Mão (Claim)
// ---------------------------------------------------------------------------

/// Patinho, este enum diz ao servidor qual tipo de requisição estamos enviando.
/// No caso do claim, o tipo será sempre "claim".
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClaimKind {
    Claim,
}

/// O pacote de dados (Payload) que enviamos no `POST /api/tropic-of-cancer/claim`.
/// É o formulário de registro do dispositivo! Ele envia as informações coletadas pelo instalador,
/// os dados básicos do computador, a nossa chave pública Ed25519 recém-gerada,
/// e a informação sobre a chave de localização que conhecemos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequest {
    pub kind: ClaimKind,
    /// Código temporário de autorização gerado pelo portal administrativa para esta instalação.
    pub authorization_code: String,
    /// Patrimônio físico do computador (ex: TI-0234).
    pub asset_tag: String,
    /// Matrícula do funcionário que usará esta máquina (ex: 12345).
    pub employee_registration: String,
    /// Dados básicos identificadores do computador (modelo, fabricante, etc.).
    pub device: DeviceInfo,
    /// Informações sobre o próprio executável do agente (versão, plataforma).
    pub agent: AgentInfo,
    /// A nossa chave pública Ed25519. O servidor vai guardar ela para verificar
    /// todas as assinaturas que enviarmos a partir de agora!
    pub signing_public_key: SigningPublicKey,
    /// Detalhes sobre a chave de localização que o agente conhece, para o servidor saber se está atualizada.
    pub location_encryption: LocationEncryptionInfo,
    /// O momento exato (carimbado com data e hora UTC) em que coletamos estes dados.
    #[serde(with = "time::serde::rfc3339")]
    pub collected_at: OffsetDateTime,
}

/// A chave pública Ed25519 do dispositivo.
/// Enviamos ela no claim para o servidor nos cadastrar. Ela possui:
/// - `key_id`: identificador da chave (pode ser vazio no claim inicial e o servidor gera um).
/// - `algorithm`: "ed25519" (o algoritmo matemático que escolhemos).
/// - `encoding`: "base64url" (o formato em que os bytes da chave foram convertidos para texto).
/// - `value`: a chave em si convertida em string Base64URL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SigningPublicKey {
    pub key_id: String,
    pub algorithm: String,
    pub encoding: String,
    pub value: String,
}

/// Informações sobre a chave pública de criptografia de localização embutida no agente.
/// Serve para o servidor saber com qual chave de geolocalização estamos trancando as coisas!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocationEncryptionInfo {
    pub key_id: String,
    pub algorithm: String,
}

/// A resposta que o servidor nos devolve após fazermos um `POST /api/tropic-of-cancer/claim` de sucesso.
/// Ela contém:
/// - `ok`: verdadeiro se deu tudo certo.
/// - `device_id`: o ID definitivo que o servidor deu para este computador.
/// - `device_key_id`: o ID associado à chave pública que registramos.
/// - `heartbeat_interval_seconds`: a frequência com que o servidor quer que enviemos heartbeats (ex: 15 minutos).
/// - `server_time`: o horário atualizado do servidor (para sabermos se o relógio da máquina local está certo).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResponse {
    pub ok: bool,
    pub device_id: String,
    pub device_key_id: String,
    pub heartbeat_interval_seconds: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// Batimento Cardíaco (Heartbeat)
// ---------------------------------------------------------------------------

/// Patinho, este enum serve para identificar a mensagem de batimento cardíaco ("heartbeat").
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HeartbeatKind {
    Heartbeat,
}

/// O pacote de dados completo enviado em cada batimento cardíaco assinado.
/// Ele reúne todo o inventário atualizado da máquina: hardware, sistema, rede, bateria,
/// e opcionalmente a geolocalização criptografada.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub kind: HeartbeatKind,
    /// O ID exclusivo do dispositivo (gerado pelo claim).
    pub device_id: String,
    /// Identificação do hardware (serial, hostname, fabricante).
    pub device: DeviceInfo,
    /// Vínculo atual de patrimônio e funcionário responsável.
    pub assignment: AssignmentInfo,
    /// Informações sobre a versão do executável do agente.
    pub agent: AgentInfo,
    /// Dados de sistema operacional, CPU e memória.
    pub system: SystemInfo,
    /// Placas de rede ativas, MAC, IPs e conexão sem fio.
    pub network: NetworkInfo,
    /// Informações da bateria física do dispositivo.
    pub battery: BatteryInfo,
    /// Geolocalização protegida criptograficamente (opcional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<EncryptedLocation>,
    /// Momento exato em que o inventário foi lido na máquina local.
    #[serde(with = "time::serde::rfc3339")]
    pub collected_at: OffsetDateTime,
}

impl HeartbeatRequest {
    /// Patinho, esta função é uma proteção muito legal!
    /// Antes de enviarmos um heartbeat, precisamos garantir que temos dados consistentes sobre
    /// a identidade da máquina (um "Identificador Forte").
    /// O que consideramos um identificador forte?
    /// 1. Conhecer o Número de Série física da máquina (`serial_number`).
    /// 2. OU ter pelo menos um endereço MAC de rede física válido (que não seja tudo zero).
    /// 3. OU ter um "fingerprint" de hardware confiável (hostname válido, fabricante e modelo preenchidos).
    ///
    /// Se não tivermos nenhum desses, a máquina é um "fantasma" digital e não podemos confiar nela!
    pub fn has_strong_identifier(&self) -> bool {
        let has_serial = has_text(self.device.serial_number.as_deref());
        let has_valid_mac = self
            .network
            .mac_addresses
            .iter()
            .any(|mac| is_valid_mac(mac));
        let has_device_fingerprint = has_text(Some(&self.device.hostname))
            && !self.device.hostname.eq_ignore_ascii_case("unknown")
            && has_text(self.device.manufacturer.as_deref())
            && has_text(self.device.model.as_deref());

        has_serial || has_valid_mac || has_device_fingerprint
    }
}

/// O vínculo atual entre o funcionário e o patrimônio da máquina.
/// Ela indica a matrícula cadastrada e a origem dessa informação (`installer` para o agente).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentInfo {
    pub employee_registration: String,
    pub source: String,
}

/// O envelope que carrega a localização geográfica do dispositivo criptografada.
/// Como vimos no `location.rs`, os dados reais de latitude e longitude não viajam abertos!
/// Eles são trancados e encapsulados aqui.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedLocation {
    pub encrypted: bool,
    pub key_id: String,
    pub algorithm: String,
    pub ciphertext: String,
    pub encapsulated_key: String,
    pub nonce: Option<String>,
}


// ---------------------------------------------------------------------------
// Tipos Compartilhados (Shared types)
// ---------------------------------------------------------------------------

/// Patinho, este struct traz os dados identificadores físicos do computador:
/// - `hostname`: o nome do computador na rede local.
/// - `serial_number`: o número de série de fábrica (geralmente lido da BIOS/SMBIOS).
/// - `manufacturer`: quem fabricou o computador (ex: "Dell Inc.").
/// - `model`: qual o modelo da máquina (ex: "Latitude 3420").
/// - `asset_tag`: o número de patrimônio físico colado no gabinete.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub hostname: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub asset_tag: Option<String>,
}

/// Detalhes sobre o nosso agente de software em execução:
/// - `version`: qual a versão dele (ex: "0.1.0").
/// - `platform`: em qual sistema operacional ele está rodando (Windows, Linux, macOS).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub version: String,
    pub platform: AgentPlatform,
}

/// Patinho, estes são os sistemas operacionais que o nosso agente sabe identificar!
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentPlatform {
    Windows,
    Linux,
    Macos,
    Unknown,
}

/// Dados detalhados de hardware e desempenho do sistema operacional:
/// - `os_name`: nome do sistema (ex: "Windows 10 Pro").
/// - `os_version`: versão do kernel ou compilação do sistema.
/// - `os_build`: número da compilação.
/// - `cpu_brand`: o modelo do processador principal (ex: "Intel Core i7-1185G7").
/// - `total_memory_bytes`: quanta memória RAM a máquina tem no total.
/// - `used_memory_bytes`: quanta RAM está ocupada neste momento.
/// - `uptime_seconds`: há quantos segundos o computador está ligado e rodando!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_build: Option<String>,
    pub cpu_brand: Option<String>,
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub uptime_seconds: u64,
}

/// Detalhes de rede da máquina para sabermos como ela está conectada ao mundo!
/// - `network_type`: se é WiFi, cabo, VPN, etc.
/// - `private_ips`: lista de endereços IP privados locais (ex: 192.168.1.15).
/// - `mac_addresses`: os endereços físicos permanentes das placas de rede (ex: "00:11:22...").
/// - `gateway_ip`: o endereço IP do roteador local.
/// - `ssid`: nome da rede sem fio em que a máquina está conectada.
/// - `bssid_hash`: hash do MAC do roteador WiFi para proteger a privacidade, mas identificar redes comuns!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub network_type: NetworkType,
    pub private_ips: Vec<String>,
    pub mac_addresses: Vec<String>,
    pub gateway_ip: Option<String>,
    pub ssid: Option<String>,
    pub bssid_hash: Option<String>,
}

/// Os tipos de conexão de rede reconhecidos!
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Wifi,
    Ethernet,
    Vpn,
    Cellular,
    Unknown,
}

/// Informações rápidas sobre a saúde de energia do notebook!
/// - `percent`: porcentagem atual de carga (0 a 100).
/// - `is_charging`: verdadeiro se a máquina está ligada na tomada e recarregando!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub percent: Option<u8>,
    pub is_charging: Option<bool>,
}

/// A resposta do servidor ao receber um batimento cardíaco.
/// Ele nos confirma que deu tudo certo (`ok: true`), ecoa o ID do dispositivo,
/// e devolve o carimbo de data/hora em que a requisição foi recebida e processada!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub device_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub server_time: OffsetDateTime,
}

// ---------------------------------------------------------------------------
// Contratos Futuros (Mantidos para compatibilidade com o servidor)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Patinho, este struct foi preparado para quando implementarmos o envio de mensagens do portal do administrador
/// direto para o computador do funcionário (Fases futuras). O agente do MVP não faz o processamento disso.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePollResponse {
    pub messages: Vec<AgentMessage>,
}

#[allow(dead_code)]
/// Detalhes de uma mensagem enviada pela empresa (título, corpo em formato HTML, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub title: String,
    pub html_body: String,
}

#[allow(dead_code)]
/// A confirmação (Acknowledge) que o agente envia ao servidor dizendo: "Sim, o funcionário leu esta mensagem"!
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAckRequest {
    pub message_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub acknowledged_at: OffsetDateTime,
}


// ---------------------------------------------------------------------------
// Funções Ajudantes (Helpers)
// ---------------------------------------------------------------------------

/// Patinho, esta função ajuda o agente a descobrir em qual sistema operacional
/// ele está sendo executado neste momento.
/// Usamos as diretivas condicionais do compilador do Rust (`cfg!`) para tomar essa decisão
/// de forma super rápida em tempo de compilação!
pub fn current_platform() -> AgentPlatform {
    if cfg!(windows) {
        AgentPlatform::Windows
    } else if cfg!(target_os = "macos") {
        AgentPlatform::Macos
    } else if cfg!(target_os = "linux") {
        AgentPlatform::Linux
    } else {
        AgentPlatform::Unknown
    }
}

/// Um ajudante simples que confere se uma `Option<&str>` possui algum texto
/// relevante (ou seja, se não é nula nem está cheia de espaços em branco!).
fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

/// Uma função super legal para validar se um endereço MAC físico de placa de rede é legítimo!
/// Um MAC válido precisa:
/// 1. Estar no formato de 6 grupos hexadecimais separados por dois pontos `:` (ex: "00:11:22:33:44:55").
/// 2. Não ser um endereço de "teste" ou nulo composto apenas por zeros ("00:00:00:00:00:00").
/// 3. Conter apenas caracteres hexadecimais válidos (0-9, a-f, A-F).
fn is_valid_mac(value: &str) -> bool {
    let mut all_zero = true;
    let mut parts = 0usize;
    for part in value.split(':') {
        parts += 1;
        if part.len() != 2 || !part.chars().all(|char| char.is_ascii_hexdigit()) {
            return false;
        }
        if part != "00" {
            all_zero = false;
        }
    }

    parts == 6 && !all_zero
}

// ---------------------------------------------------------------------------
// Cabeçalhos de Assinatura (Signature headers)
// ---------------------------------------------------------------------------

/// Patinho, este struct especial reúne todos os cabeçalhos de assinatura criptográfica
/// exigidos pelo nosso protocolo de segurança no envio de batimentos cardíacos!
/// Quando enviamos a requisição HTTP `POST /api/tropic-of-cancer/heartbeat`, nós injetamos
/// estas cinco strings nos cabeçalhos da mensagem:
/// - `device_id`: prova quem somos nós.
/// - `device_key_id`: indica qual chave pública no servidor deve ser usada para verificar.
/// - `timestamp`: evita ataques de atraso (o servidor confere se a requisição é recente).
/// - `nonce`: número único gerado aleatoriamente para evitar ataques de repetição.
/// - `signature`: a assinatura Ed25519 de 64 bytes (em Base64URL) calculada sobre a String Canônica!
pub struct SignatureHeaders {
    pub device_id: String,
    pub device_key_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub signature: String,
}

// ---------------------------------------------------------------------------
// Suíte de Testes (Tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn heartbeat_request_should_serialize_to_new_contract() {
        // Patinho, este teste confere se o nosso struct `HeartbeatRequest` é serializado em JSON
        // seguindo exatamente o formato camelCase exigido pela API do backend!
        let request = HeartbeatRequest {
            kind: HeartbeatKind::Heartbeat,
            device_id: "dev_01J".to_string(),
            device: DeviceInfo {
                hostname: "NB-FISCAL-023".to_string(),
                serial_number: Some("ABC123".to_string()),
                manufacturer: Some("Dell Inc.".to_string()),
                model: Some("Latitude 3420".to_string()),
                asset_tag: Some("TI-0234".to_string()),
            },
            assignment: AssignmentInfo {
                employee_registration: "12345".to_string(),
                source: "installer".to_string(),
            },
            agent: AgentInfo {
                version: "0.1.0".to_string(),
                platform: AgentPlatform::Windows,
            },
            system: SystemInfo {
                os_name: Some("Windows".to_string()),
                os_version: Some("10.0.19045".to_string()),
                os_build: Some("19045".to_string()),
                cpu_brand: Some("Intel(R) Core(TM) i5".to_string()),
                total_memory_bytes: 17_179_869_184,
                used_memory_bytes: 8_589_934_592,
                uptime_seconds: 123_456,
            },
            network: NetworkInfo {
                network_type: NetworkType::Wifi,
                private_ips: vec!["192.168.10.43".to_string()],
                mac_addresses: vec!["00:11:22:33:44:55".to_string()],
                gateway_ip: Some("192.168.10.1".to_string()),
                ssid: Some("Elinsa-ADM".to_string()),
                bssid_hash: None,
            },
            battery: BatteryInfo {
                percent: Some(88),
                is_charging: Some(true),
            },
            location: None,
            collected_at: OffsetDateTime::from_unix_timestamp(1_780_228_800).unwrap(),
        };

        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["kind"], "heartbeat");
        assert_eq!(value["deviceId"], "dev_01J");
        assert_eq!(value["assignment"]["employeeRegistration"], "12345");
        assert_eq!(value["assignment"]["source"], "installer");
        assert_eq!(value["device"]["assetTag"], "TI-0234");
        assert!(value.get("location").is_none());
    }

    #[test]
    fn claim_request_should_serialize_correctly() {
        // Testamos se o struct de Claim (aperto de mão) também serializa lindamente!
        let request = ClaimRequest {
            kind: ClaimKind::Claim,
            authorization_code: "CAN-9F2K-8R".to_string(),
            asset_tag: "TI-0234".to_string(),
            employee_registration: "12345".to_string(),
            device: DeviceInfo {
                hostname: "ARGENTUM".to_string(),
                serial_number: Some("4J6JG2G4".to_string()),
                manufacturer: Some("Dell Inc.".to_string()),
                model: Some("Dell Pro 16 PC16250".to_string()),
                asset_tag: None,
            },
            agent: AgentInfo {
                version: "0.1.0".to_string(),
                platform: AgentPlatform::Windows,
            },
            signing_public_key: SigningPublicKey {
                key_id: "local_generated".to_string(),
                algorithm: "ed25519".to_string(),
                encoding: "base64url".to_string(),
                value: "PUBLIC_KEY_BASE64URL".to_string(),
            },
            location_encryption: LocationEncryptionInfo {
                key_id: "loc_2026_01".to_string(),
                algorithm: "HPKE-X25519-HKDF-SHA256-AES256GCM".to_string(),
            },
            collected_at: OffsetDateTime::from_unix_timestamp(1_780_228_800).unwrap(),
        };

        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["kind"], "claim");
        assert_eq!(value["authorizationCode"], "CAN-9F2K-8R");
        assert_eq!(value["signingPublicKey"]["algorithm"], "ed25519");
        assert_eq!(value["locationEncryption"]["keyId"], "loc_2026_01");
    }

    #[test]
    fn claim_response_should_deserialize() {
        // Testamos se a resposta do claim vinda do servidor (JSON) é lida corretamente
        // pela nossa lógica Rust.
        let json = json!({
            "ok": true,
            "deviceId": "dev_01J",
            "deviceKeyId": "devkey_01J",
            "heartbeatIntervalSeconds": 900,
            "serverTime": "2026-06-01T12:00:01.000Z"
        });

        let response: ClaimResponse = serde_json::from_value(json).unwrap();
        assert!(response.ok);
        assert_eq!(response.device_id, "dev_01J");
        assert_eq!(response.device_key_id, "devkey_01J");
        assert_eq!(response.heartbeat_interval_seconds, 900);
    }

    #[test]
    fn heartbeat_request_should_accept_valid_mac_as_strong_identifier() {
        // Garantimos que uma placa de rede com MAC válido é suficiente para passar no teste de segurança
        // de identificador forte!
        let request = heartbeat_request_for_tests();
        assert!(request.has_strong_identifier());
    }

    #[test]
    fn heartbeat_request_should_reject_missing_strong_identifier() {
        // Garantimos que se o computador não tiver MAC válido, nem serial, nem hostname/fabricante conhecidos,
        // ele é rejeitado educadamente por falta de identificação!
        let mut request = heartbeat_request_for_tests();
        request.device.hostname = "unknown".to_string();
        request.device.serial_number = None;
        request.device.manufacturer = None;
        request.device.model = None;
        request.network.mac_addresses = vec!["00:00:00:00:00:00".to_string()];

        assert!(!request.has_strong_identifier());
    }

    fn heartbeat_request_for_tests() -> HeartbeatRequest {
        HeartbeatRequest {
            kind: HeartbeatKind::Heartbeat,
            device_id: "dev_test".to_string(),
            device: DeviceInfo {
                hostname: "NB-FISCAL-023".to_string(),
                serial_number: Some("ABC123".to_string()),
                manufacturer: Some("Dell Inc.".to_string()),
                model: Some("Latitude 3420".to_string()),
                asset_tag: Some("TI-0234".to_string()),
            },
            assignment: AssignmentInfo {
                employee_registration: "12345".to_string(),
                source: "installer".to_string(),
            },
            agent: AgentInfo {
                version: "0.1.0".to_string(),
                platform: AgentPlatform::Windows,
            },
            system: SystemInfo {
                os_name: Some("Windows".to_string()),
                os_version: Some("10.0.19045".to_string()),
                os_build: Some("19045".to_string()),
                cpu_brand: Some("Intel(R) Core(TM) i5".to_string()),
                total_memory_bytes: 17_179_869_184,
                used_memory_bytes: 8_589_934_592,
                uptime_seconds: 123_456,
            },
            network: NetworkInfo {
                network_type: NetworkType::Wifi,
                private_ips: vec!["192.168.10.43".to_string()],
                mac_addresses: vec!["00:11:22:33:44:55".to_string()],
                gateway_ip: Some("192.168.10.1".to_string()),
                ssid: Some("Elinsa-ADM".to_string()),
                bssid_hash: None,
            },
            battery: BatteryInfo {
                percent: Some(88),
                is_charging: Some(true),
            },
            location: None,
            collected_at: OffsetDateTime::from_unix_timestamp(1_780_228_800).unwrap(),
        }
    }
}

