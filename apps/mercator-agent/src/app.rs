use std::time::Duration;

use anyhow::{Context, Result, bail};
use tracing::{error, info, warn};

use crate::collector;
use crate::config::{self, AgentConfig};
use crate::crypto::signing;
use crate::embedded;
use crate::security::mask_secret;
use crate::types::{
    AgentInfo, AssignmentInfo, ClaimKind, ClaimRequest, HeartbeatKind, HeartbeatRequest,
    LocationEncryptionInfo, SignatureHeaders, SigningPublicKey, current_platform,
};

/// Patinho, estes são os tempos de espera em segundos para o nosso sistema de recuo (Backoff).
/// Se o agente tentar enviar um heartbeat ao servidor e falhar (por exemplo, porque a internet caiu),
/// nós não queremos ficar bombardeando o servidor a cada segundo!
/// Em vez disso, nós esperamos tempos progressivamente maiores: 60s, 120s, 300s, 600s, e finalmente 900s.
/// Assim que a internet voltar e o envio der certo, o contador de falhas zera e voltamos ao intervalo normal!
const BACKOFF_SECONDS: [u64; 5] = [60, 120, 300, 600, 900];

// ---------------------------------------------------------------------------
// Comandos do Aplicativo
// ---------------------------------------------------------------------------

/// Inicia o agente em primeiro plano (foreground).
/// Ele apenas chama a função principal `run_loop` passando um sinalizador especial
/// que avisa quando o usuário pressionou `Ctrl+C` para encerrar o programa de forma graciosa!
pub async fn run_foreground() -> Result<()> {
    info!("starting mercator-agent in foreground");
    run_loop(shutdown_on_ctrl_c()).await
}

/// O laço de repetição principal (Heartbeat Loop) do agente.
///
/// **Como ele funciona, patinho?**
/// 1. Ele carrega a configuração salva localmente. Se não estiver configurado, para com um erro instrutivo.
/// 2. Valida se o claim foi concluído e se temos chaves criptográficas prontas.
/// 3. Faz o primeiro envio de batimento imediatamente (`send_heartbeat_from_config`).
/// 4. Entra em um loop eterno monitorando duas coisas ao mesmo tempo usando `tokio::select!`:
///    - O sinal de desligamento (se vier, para imediatamente e sai de fininho).
///    - O timer de espera do próximo envio.
///
/// Se o timer disparar, o agente lê o inventário, assina e envia.
/// Se der erro, incrementamos o contador de falhas para aplicar o tempo de espera do Backoff!
pub async fn run_loop(shutdown: impl std::future::Future<Output = ()>) -> Result<()> {
    let config = config::load_config().context(
        "agent is not configured; run `mercator-agent configure --asset-tag <TAG> --employee-registration <REG> --authorization-code <CODE>` first",
    )?;
    config.validate_for_heartbeat()?;
    AgentConfig::validate_server_url()?;

    let mut failures = 0usize;
    // Primeiro batimento cardíaco disparado imediatamente ao iniciar!
    send_heartbeat_from_config(&config).await?;

    tokio::pin!(shutdown);
    loop {
        // Se não tivemos falhas recentes, o tempo de espera é o configurado pelo servidor (ex: 15min).
        // Se falhou, pegamos o tempo da nossa tabela de Backoff progressivo!
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
                        failures = 0; // Êba! Sucesso! Reseta o contador de falhas.
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

/// Salva o patrimônio físico e a matrícula do funcionário na máquina local.
/// O código de autorização é consumido depois pelo comando `claim`.
pub fn configure(
    asset_tag: &str,
    employee_registration: &str,
    _authorization_code: &str,
) -> Result<()> {
    // Cria uma nova configuração e a valida de forma básica.
    let config = AgentConfig::new_from_provisioning(
        asset_tag.to_string(),
        employee_registration.to_string(),
    )?;
    // Grava o JSON formatado no diretório de dados padrão.
    config::save_config(&config)?;

    println!("Mercator Agent configurado.");
    println!("  assetTag: {}", config.asset_tag);
    println!("  employeeRegistration: {}", config.employee_registration);
    println!(
        "  serverUrl (embutido): {}",
        mask_secret(embedded::SERVER_URL)
    );
    println!();
    println!("Execute `mercator-agent claim` para registrar o dispositivo.");
    Ok(())
}


/// Patinho, este é o comando `claim`.
/// Ele é o aperto de mão inicial super seguro do agente com o servidor Mercator!
///
/// **Como funciona o processo de claim?**
/// 1. Nós validamos as variáveis de compilação essenciais (URL do servidor, chave de handshake).
/// 2. Verificamos se já temos uma chave privada Ed25519 salva no HD.
///    - Se já existir, nós a carregamos!
///    - Se não existir (primeira vez), geramos um novo par de chaves e salvamos no arquivo seguro.
/// 3. Lemos os dados básicos do hardware do computador (inventário mínimo).
/// 4. Montamos a estrutura de `ClaimRequest`, passando a nossa chave pública em Base64URL.
/// 5. Enviamos para o endpoint `POST /claim` usando a nossa chave de handshake como credencial.
/// 6. Se o servidor aprovar o claim, ele nos devolve:
///    - Nosso `deviceId` exclusivo.
///    - O ID da nossa chave pública.
///    - O intervalo de tempo desejado para heartbeats.
/// 7. Nós salvamos tudo isso de volta na nossa configuração e comemoramos!
pub async fn claim(authorization_code: &str) -> Result<()> {
    embedded::validate_embedded_config()
        .map_err(|message| anyhow::anyhow!(message))?;

    let mut config = config::load_config().context(
        "agent is not configured; run `mercator-agent configure` first",
    )?;

    // Procura a nossa chave privada no diretório padrão de chaves.
    let key_path = config::signing_private_key_path();
    let keypair = if key_path.exists() {
        // Se ela já está lá, nós apenas lemos ela!
        info!("loading existing signing keypair");
        let sk = signing::load_private_key(&key_path)
            .context("failed to load existing private key")?;
        signing::DeviceSigningKeyPair {
            verifying_key: sk.verifying_key(),
            signing_key: sk,
        }
    } else {
        // Se for a primeira vez na máquina, criamos uma nova do zero!
        info!("generating new Ed25519 signing keypair");
        let kp = signing::generate_keypair();
        config::ensure_keys_dir()?;
        signing::save_private_key(&kp.signing_key, &key_path)
            .context("failed to save private key")?;
        kp
    };

    // Convertemos a chave de verificação pública para uma string Base64URL amigável.
    let public_key_b64 = signing::public_key_base64url(&keypair.verifying_key);

    // Coleta dados básicos do computador para o servidor saber quem somos nós fisicamente!
    let snapshot = collector::collect_system_info();

    let location_encryption = if embedded::LOCATION_PUBLIC_KEY_ID.is_empty() || embedded::LOCATION_PUBLIC_KEY.is_empty() {
        None
    } else {
        Some(LocationEncryptionInfo {
            key_id: embedded::LOCATION_PUBLIC_KEY_ID.to_string(),
            algorithm: "HPKE-X25519-HKDF-SHA256-AES256GCM".to_string(),
        })
    };

    // Monta o pacote de requisição do claim.
    let claim_request = ClaimRequest {
        kind: ClaimKind::Claim,
        authorization_code: authorization_code.to_string(),
        asset_tag: config.asset_tag.clone(),
        employee_registration: config.employee_registration.clone(),
        device: snapshot.device,
        agent: AgentInfo {
            version: config.agent_version.clone(),
            platform: current_platform(),
        },
        signing_public_key: SigningPublicKey {
            key_id: None, // O servidor criará e atribuirá um ID para a chave pública.
            algorithm: "ed25519".to_string(),
            encoding: "base64url".to_string(),
            value: public_key_b64,
        },
        location_encryption,
        collected_at: time::OffsetDateTime::now_utc(),
    };

    println!("Enviando claim para {}...", mask_secret(embedded::SERVER_URL));

    // Faz a chamada de rede usando reqwest.
    let response = crate::api::send_claim(
        embedded::SERVER_URL,
        embedded::HANDSHAKE_KEY,
        &claim_request,
    )
    .await
    .context("claim request failed")?;

    // Atualiza a nossa ficha cadastral local com o retorno vitorioso do servidor!
    config.mark_claimed(
        response.device_id.clone(),
        response.device_key_id.clone(),
        response.heartbeat_interval_seconds,
    );
    config::save_config(&config)?;

    println!("Claim bem-sucedido!");
    println!("  deviceId: {}", response.device_id);
    println!("  deviceKeyId: {}", response.device_key_id);
    println!(
        "  heartbeatIntervalSeconds: {}",
        response.heartbeat_interval_seconds
    );
    println!();
    println!("Execute `mercator-agent heartbeat-once` para enviar o primeiro heartbeat.");

    info!(
        device_id = %response.device_id,
        device_key_id = %response.device_key_id,
        "claim completed"
    );

    Ok(())
}

/// Patinho, este é o comando `heartbeat-once`.
/// Ele lê todo o inventário atual da máquina e faz um envio imediato de heartbeat
/// assinado criptograficamente! É fantástico para verificar se tudo está funcionando
/// e testar a comunicação.
pub async fn heartbeat_once() -> Result<()> {
    let config = config::load_config().context("cannot send heartbeat without config")?;
    config.validate_for_heartbeat()?;
    AgentConfig::validate_server_url()?;

    // Coleta tudo, monta a requisição JSON, e calcula a assinatura Ed25519!
    let (payload, sig) = build_signed_heartbeat(&config)?;

    println!("hostname: {}", payload.device.hostname);
    println!(
        "serial: {}",
        payload
            .device
            .serial_number
            .as_deref()
            .unwrap_or("unavailable")
    );
    println!("deviceId: {}", payload.device_id);

    // Envia o batimento assinado.
    let result = crate::api::send_heartbeat(embedded::SERVER_URL, &payload, &sig).await;

    match result {
        Ok(response) => {
            println!("status: sent");
            println!("deviceId: {}", response.device_id);
            Ok(())
        }
        Err(error) => {
            println!("status: failed");
            Err(error.into())
        }
    }
}

/// Patinho, este é o comando `status`.
/// Ele carrega a configuração atual salva no HD e exibe em formato JSON bonito
/// na tela. É excelente para administradores e suporte diagnosticarem se o agente
/// está provisionado e com o claim completo!
pub fn status() -> Result<()> {
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


// ---------------------------------------------------------------------------
// Ajudantes Internos (Internal helpers)
// ---------------------------------------------------------------------------

/// Patinho, esta função auxilia o loop de heartbeats a enviar o batimento cardíaco.
/// Ela:
/// 1. Constrói o payload de heartbeat e o assina.
/// 2. Confere se o pacote possui um "Identificador Forte" para segurança anti-fraude.
/// 3. Faz a chamada de rede para enviar o heartbeat.
/// 4. Se der tudo certo, registra uma mensagem informativa nos logs!
async fn send_heartbeat_from_config(config: &AgentConfig) -> Result<()> {
    let (payload, sig) = build_signed_heartbeat(config)?;

    // Proteção de segurança essencial! Só aceitamos pacotes com identidades fortes.
    if !payload.has_strong_identifier() {
        bail!(
            "heartbeat payload lacks a strong device identifier: expected serialNumber, valid MAC, or hostname + manufacturer + model"
        );
    }

    crate::api::send_heartbeat(embedded::SERVER_URL, &payload, &sig)
        .await
        .context("heartbeat request failed")?;

    info!(
        hostname = %payload.device.hostname,
        device_id = %payload.device_id,
        private_ip_count = payload.network.private_ips.len(),
        "heartbeat sent"
    );
    Ok(())
}

/// Patinho, este método é onde toda a mágica da montagem do batimento cardíaco assinado acontece!
/// É como colocar todas as peças de um quebra-cabeça juntas:
///
/// **Passo a Passo da Montagem:**
/// 1. Pegamos o `deviceId` e o `deviceKeyId` guardados localmente.
/// 2. Lemos as informações de hardware atuais do sistema (CPU, memória RAM, uptime).
/// 3. Lemos as informações de rede (SSID, IPs, MACs).
/// 4. Lemos as informações da bateria.
/// 5. Montamos o struct `HeartbeatRequest`.
/// 6. Carregamos a nossa chave privada do disco.
/// 7. Convertemos a requisição inteira em bytes JSON para tirarmos a impressão digital SHA-256 (`body_hash`).
/// 8. Geramos um Nonce UUID v4 novinho em folha e pegamos o timestamp atual no formato padrão RFC3339.
/// 9. Construímos a String Canônica estrita com todas essas informações.
/// 10. Assinamos essa String Canônica com a chave privada, gerando a nossa assinatura digital definitiva!
/// 11. Devolvemos a requisição JSON preenchida e os cabeçalhos de assinatura criptográfica.
fn build_signed_heartbeat(
    config: &AgentConfig,
) -> Result<(HeartbeatRequest, SignatureHeaders)> {
    let device_id = config
        .device_id
        .as_deref()
        .context("deviceId is missing from config; run `mercator-agent claim` first")?;
    let device_key_id = config
        .device_key_id
        .as_deref()
        .context("deviceKeyId is missing from config; run `mercator-agent claim` first")?;

    // Coleta todos os sensores locais!
    let snapshot = collector::collect_system_info();
    let network = collector::collect_network_info();
    let battery = collector::collect_battery_info();

    let payload = HeartbeatRequest {
        kind: HeartbeatKind::Heartbeat,
        device_id: device_id.to_string(),
        device: crate::types::DeviceInfo {
            asset_tag: Some(config.asset_tag.clone()),
            ..snapshot.device
        },
        assignment: AssignmentInfo {
            employee_registration: config.employee_registration.clone(),
            source: "installer".to_string(),
        },
        agent: AgentInfo {
            version: config.agent_version.clone(),
            platform: current_platform(),
        },
        system: snapshot.system,
        network,
        battery,
        location: None, // TODO(fase-4): criptografia de localização HPKE
        collected_at: time::OffsetDateTime::now_utc(),
    };

    // --- Assinatura do Pacote ---
    // 1. Carrega a chave privada
    let key_path = config::signing_private_key_path();
    let signing_key = signing::load_private_key(&key_path)
        .context("failed to load signing private key; was `claim` completed?")?;

    // 2. Transforma o payload em JSON para gerar o hash do corpo
    let body_bytes = serde_json::to_vec(&payload)
        .context("failed to serialize heartbeat payload")?;
    let body_hash = signing::sha256_base64url(&body_bytes);
    
    // 3. Pega o timestamp atualizado em UTC
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
        
    // 4. Gera o Nonce aleatório de uso único
    let nonce = signing::generate_nonce();

    // 5. Monta a string canônica padrão
    let canonical =
        signing::build_canonical_string(device_id, device_key_id, &timestamp, &nonce, &body_hash);
        
    // 6. Calcula a assinatura Ed25519 final!
    let signature = signing::sign_canonical(&canonical, &signing_key);

    let sig_headers = SignatureHeaders {
        device_id: device_id.to_string(),
        device_key_id: device_key_id.to_string(),
        timestamp,
        nonce,
        signature,
    };

    Ok((payload, sig_headers))
}

/// Um ajudante simples que escuta o sinal de encerramento Ctrl+C e avisa o laço principal.
async fn shutdown_on_ctrl_c() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to listen for Ctrl+C");
    }
}

