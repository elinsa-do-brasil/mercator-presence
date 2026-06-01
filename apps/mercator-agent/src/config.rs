use std::fs;
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::security::{SecurityError, mask_secret, validate_server_url};

pub const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: u64 = 900;
pub const DEFAULT_AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Olá, patinho! Este é o struct `AgentConfig`.
/// Pense nele como a ficha cadastral do nosso agente que fica salva localmente no HD.
/// Ela guarda o estado do dispositivo: se ele foi provisionado pelo administrador, se o servidor
/// já o aceitou e deu um ID de dispositivo exclusivo, e qual o intervalo de tempo em que devemos
/// enviar os batimentos cardíacos (heartbeats).
///
/// **Mudança Importante de Arquitetura:**
/// - A URL do servidor não é mais salva aqui! Ela vem fixa de fábrica (`embedded::SERVER_URL`).
/// - A chave de API antiga (api_key plana) sumiu! Agora usamos a chave de handshake para o primeiro contato
///   e assinaturas criptográficas Ed25519 a partir dali!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    // --- Provisionamento (preenchido na fase `configure`) ---
    /// Número de patrimônio do ativo físico da empresa (ex: "TI-0234").
    pub asset_tag: String,
    /// Matrícula do funcionário associado (ex: "12345").
    pub employee_registration: String,

    // --- Claim (preenchido após o aperto de mão bem-sucedido com o servidor) ---
    /// ID exclusivo atribuído a este computador pelo servidor Mercator.
    pub device_id: Option<String>,
    /// Identificador da chave criptográfica que registramos no servidor.
    pub device_key_id: Option<String>,

    /// Frequência (em segundos) com que o agente enviará o inventário. O padrão é 900 segundos (15 minutos).
    #[serde(default = "default_heartbeat_interval_seconds")]
    pub heartbeat_interval_seconds: u64,
    /// Versão do agente instalada.
    #[serde(default = "default_agent_version")]
    pub agent_version: String,

    // --- Metadados Extras ---
    /// Data e hora em que a configuração inicial foi feita.
    pub provisioned_at: Option<String>,
    /// Onde a chave privada de assinatura está guardada (no MVP, "plaintext").
    pub signing_key_storage: Option<String>,
    /// O ID da chave de localização em uso na compilação.
    pub location_public_key_id: Option<String>,
}

/// Patinho, este é o struct `DisplayConfig`.
/// Ele é uma versão especial da nossa configuração usada puramente para amostragem amigável!
/// Quando o usuário roda `mercator-agent status`, nós preenchemos este struct e exibimos na tela.
/// Note que ele mascara segredos (como a URL do servidor, se contiver tokens de acesso) e mostra
/// caminhos completos de diretórios para facilitar o diagnóstico de suporte!
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DisplayConfig {
    pub server_url: String,
    pub asset_tag: String,
    pub employee_registration: String,
    pub device_id: Option<String>,
    pub device_key_id: Option<String>,
    pub heartbeat_interval_seconds: u64,
    pub agent_version: String,
    pub is_provisioned: bool,
    pub is_claimed: bool,
    pub config_path: String,
    pub log_path: String,
    pub keys_dir: String,
}

/// Todos os possíveis erros que podem acontecer quando lidamos com configuração, patinho!
/// Como arquivos corrompidos, permissões insuficientes, JSON inválido ou falhas de segurança.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file does not exist at {0}")]
    Missing(PathBuf),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl AgentConfig {
    /// Patinho, este método é chamado pelo comando `configure`.
    /// Ele cria uma nova instância da nossa configuração apenas com os dados de provisionamento.
    /// É o primeiro passo do ciclo de vida do agente!
    pub fn new_from_provisioning(
        asset_tag: String,
        employee_registration: String,
    ) -> Result<Self, ConfigError> {
        let config = Self {
            // Validamos que o patrimônio e a matrícula não vieram em branco!
            asset_tag: non_empty(asset_tag, "assetTag")?,
            employee_registration: non_empty(employee_registration, "employeeRegistration")?,
            device_id: None,
            device_key_id: None,
            heartbeat_interval_seconds: DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
            agent_version: DEFAULT_AGENT_VERSION.to_string(),
            provisioned_at: Some(time::OffsetDateTime::now_utc().to_string()),
            signing_key_storage: None,
            location_public_key_id: None,
        };
        config.validate_provisioned()?;
        Ok(config)
    }

    /// Patinho, este método é chamado logo após o comando `claim` ter sucesso na rede!
    /// O servidor nos deu um ID de dispositivo (`device_id`), um ID da nossa chave (`device_key_id`),
    /// e também o intervalo ideal para os batimentos. Nós marcamos esses dados no nosso config!
    pub fn mark_claimed(
        &mut self,
        device_id: String,
        device_key_id: String,
        heartbeat_interval_seconds: u64,
    ) {
        self.device_id = Some(device_id);
        self.device_key_id = Some(device_key_id);
        if heartbeat_interval_seconds > 0 {
            self.heartbeat_interval_seconds = heartbeat_interval_seconds;
        }
        self.signing_key_storage = Some("plaintext".to_string());
        self.location_public_key_id = if crate::embedded::LOCATION_PUBLIC_KEY_ID.is_empty() {
            None
        } else {
            Some(crate::embedded::LOCATION_PUBLIC_KEY_ID.to_string())
        };
    }

    /// O computador já passou pela primeira etapa de configuração (patrimônio + matrícula)?
    pub fn is_provisioned(&self) -> bool {
        !self.asset_tag.is_empty() && !self.employee_registration.is_empty()
    }

    /// O computador já completou o registro completo com o servidor (obteve deviceId + deviceKeyId)?
    pub fn is_claimed(&self) -> bool {
        self.device_id.is_some() && self.device_key_id.is_some()
    }

    /// Validação de segurança básica: confere se patrimônio e matrícula não estão vazios ou cheios de espaços em branco!
    pub fn validate_provisioned(&self) -> Result<(), ConfigError> {
        if self.asset_tag.trim().is_empty() {
            return Err(ConfigError::Invalid("assetTag cannot be empty".to_string()));
        }
        if self.employee_registration.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "employeeRegistration cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validação estrita exigida antes de rodar o loop de heartbeats.
    /// Patinho, não podemos enviar heartbeats assinados sem antes termos feito o `claim` (aperto de mão) com sucesso!
    pub fn validate_for_heartbeat(&self) -> Result<(), ConfigError> {
        self.validate_provisioned()?;
        require_present(&self.device_id, "deviceId")?;
        require_present(&self.device_key_id, "deviceKeyId")?;
        Ok(())
    }

    /// Valida se a URL do servidor embutida no binário é um endereço HTTPS/HTTP válido e seguro!
    pub fn validate_server_url() -> Result<(), ConfigError> {
        let url = crate::embedded::SERVER_URL;
        if url.is_empty() {
            return Err(ConfigError::Invalid(
                "MERCATOR_SERVER_URL is empty. Rebuild with a valid .env file.".to_string(),
            ));
        }
        validate_server_url(url)?;
        Ok(())
    }

    /// Monta a versão amigável para exibição no comando `status`.
    pub fn display(&self) -> DisplayConfig {
        DisplayConfig {
            server_url: mask_secret(crate::embedded::SERVER_URL),
            asset_tag: self.asset_tag.clone(),
            employee_registration: self.employee_registration.clone(),
            device_id: self.device_id.clone(),
            device_key_id: self.device_key_id.clone(),
            heartbeat_interval_seconds: self.heartbeat_interval_seconds,
            agent_version: self.agent_version.clone(),
            is_provisioned: self.is_provisioned(),
            is_claimed: self.is_claimed(),
            config_path: config_path().display().to_string(),
            log_path: log_path().display().to_string(),
            keys_dir: keys_dir().display().to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Persistência
// ---------------------------------------------------------------------------

/// Patinho, esta função carrega as configurações do agente a partir do caminho padrão no HD.
pub fn load_config() -> Result<AgentConfig, ConfigError> {
    load_config_from_path(&config_path())
}

/// Carrega a configuração de um caminho de arquivo específico.
/// Lemos o JSON do arquivo e convertemos de volta em um struct `AgentConfig` na memória!
pub fn load_config_from_path(path: &Path) -> Result<AgentConfig, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::Missing(path.to_path_buf()));
    }
    let raw = fs::read_to_string(path)?;
    let config = serde_json::from_str::<AgentConfig>(&raw)?;
    // Garantimos que os dados mínimos estão corretos depois de carregar.
    config.validate_provisioned()?;
    Ok(config)
}

/// Salva as configurações atuais do agente no arquivo de configuração padrão.
pub fn save_config(config: &AgentConfig) -> Result<(), ConfigError> {
    ensure_config_dir()?;
    save_config_to_path(config, &config_path())
}

/// Grava a configuração em formato JSON formatado e bonito (pretty print) no arquivo indicado.
pub fn save_config_to_path(config: &AgentConfig, path: &Path) -> Result<(), ConfigError> {
    config.validate_provisioned()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)?;
    fs::write(path, raw)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Criação e Caminho de Diretórios
// ---------------------------------------------------------------------------

/// Garante que a pasta principal do agente exista no HD (cria se necessário).
pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
    let path = base_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Garante que a pasta de logs do agente exista no HD.
pub fn ensure_log_dir() -> Result<PathBuf, ConfigError> {
    let path = log_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Garante que a pasta especial de chaves criptográficas exista no HD.
pub fn ensure_keys_dir() -> Result<PathBuf, ConfigError> {
    let path = keys_dir();
    fs::create_dir_all(&path)?;
    Ok(path)
}

/// Caminho do arquivo de configuração JSON (`config.json`).
pub fn config_path() -> PathBuf {
    base_dir().join("config.json")
}

/// Pasta onde os arquivos de logs serão gravados.
pub fn log_dir() -> PathBuf {
    base_dir().join("logs")
}

/// Caminho completo do arquivo de log do agente (`agent.log`).
pub fn log_path() -> PathBuf {
    log_dir().join("agent.log")
}

/// Pasta onde a nossa chave privada preciosa Ed25519 será salva.
pub fn keys_dir() -> PathBuf {
    base_dir().join("keys")
}

/// Caminho completo do arquivo binário contendo a chave privada (`device-signing-key.bin`).
pub fn signing_private_key_path() -> PathBuf {
    keys_dir().join("device-signing-key.bin")
}

fn default_heartbeat_interval_seconds() -> u64 {
    DEFAULT_HEARTBEAT_INTERVAL_SECONDS
}

fn default_agent_version() -> String {
    DEFAULT_AGENT_VERSION.to_string()
}

/// Ajudante de validação, patinho!
/// Garante que uma opção `Option<String>` esteja presente e que seu texto não esteja em branco.
fn require_present(value: &Option<String>, field: &str) -> Result<(), ConfigError> {
    let value = value.as_ref().ok_or_else(|| {
        ConfigError::Invalid(format!(
            "{field} is required; run `mercator-agent claim` first"
        ))
    })?;
    non_empty(value.clone(), field).map(|_| ())
}

/// Garante que uma String obrigatória não esteja em branco ou cheia de espaços vazios.
fn non_empty(value: String, field: &str) -> Result<String, ConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::Invalid(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_string())
}

/// Patinho, esta função é o coração do nosso sistema de arquivos!
/// Ela determina ONDE o agente vai gravar suas coisas.
///
/// Lógica:
/// 1. Primeiro conferimos se existe a variável de ambiente `MERCATOR_AGENT_HOME`.
///    Se ela existir, nós a respeitamos cegamente! Isso é maravilhoso para desenvolvimento local.
/// 2. No Windows (produção), nós gravamos em `C:\ProgramData\Mercator\Agent`.
///    Esta é uma pasta compartilhada do Windows, excelente para serviços do sistema rodarem sem depender de usuário.
/// 3. Em outros sistemas (como Linux e macOS), usamos diretórios padrão do sistema operacional
///    (gerenciados pela biblioteca `directories`).
fn base_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MERCATOR_AGENT_HOME") {
        return PathBuf::from(path);
    }

    #[cfg(windows)]
    {
        PathBuf::from(r"C:\ProgramData\Mercator\Agent")
    }

    #[cfg(not(windows))]
    {
        ProjectDirs::from("br.com", "Mercator", "MercatorAgent")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("mercator-agent"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_from_provisioning_should_create_config() {
        // Patinho, testamos se conseguimos iniciar uma configuração correta apenas com
        // os dados de provisionamento iniciais.
        let config = AgentConfig::new_from_provisioning(
            "TI-0234".to_string(),
            "12345".to_string(),
        )
        .unwrap();

        assert_eq!(config.asset_tag, "TI-0234");
        assert_eq!(config.employee_registration, "12345");
        assert!(config.is_provisioned());
        assert!(!config.is_claimed());
    }

    #[test]
    fn mark_claimed_should_update_fields() {
        // Testamos se após o claim os IDs do dispositivo e as chaves são atualizados!
        let mut config = AgentConfig::new_from_provisioning(
            "TI-0234".to_string(),
            "12345".to_string(),
        )
        .unwrap();

        config.mark_claimed(
            "dev_01J".to_string(),
            "devkey_01J".to_string(),
            900,
        );

        assert!(config.is_claimed());
        assert_eq!(config.device_id.as_deref(), Some("dev_01J"));
        assert_eq!(config.device_key_id.as_deref(), Some("devkey_01J"));
        assert_eq!(config.heartbeat_interval_seconds, 900);
    }

    #[test]
    fn config_round_trip_should_preserve_values() {
        // Testamos o salvamento e carregamento no disco (round trip) para garantir que
        // nenhum dado do JSON se perca no processo de persistência!
        let dir =
            std::env::temp_dir().join(format!("mercator-agent-config-test-{}", std::process::id()));
        let path = dir.join("config.json");

        let mut config = AgentConfig::new_from_provisioning(
            "TI-0234".to_string(),
            "12345".to_string(),
        )
        .unwrap();
        config.mark_claimed("dev_01J".to_string(), "devkey_01J".to_string(), 60);

        save_config_to_path(&config, &path).unwrap();
        let loaded = load_config_from_path(&path).unwrap();

        assert_eq!(loaded.asset_tag, config.asset_tag);
        assert_eq!(loaded.device_id, config.device_id);
        assert_eq!(loaded.device_key_id, config.device_key_id);
        assert_eq!(
            loaded.heartbeat_interval_seconds,
            config.heartbeat_interval_seconds
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validate_for_heartbeat_should_reject_unclaimed() {
        // Testamos se a validação impede o envio de heartbeats se o claim não foi feito.
        let config = AgentConfig::new_from_provisioning(
            "TI-0234".to_string(),
            "12345".to_string(),
        )
        .unwrap();

        assert!(config.validate_for_heartbeat().is_err());
    }

    #[test]
    fn new_from_provisioning_should_reject_empty_asset_tag() {
        // Testamos se o sistema de validação barra patrimônios em branco!
        let result = AgentConfig::new_from_provisioning("".to_string(), "12345".to_string());
        assert!(result.is_err());
    }
}

