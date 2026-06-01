//! Olá, patinho! Este é o `api.rs`.
//! Aqui mora o nosso carteiro digital! Este módulo lida com toda a comunicação de rede (HTTP)
//! do agente com o servidor Mercator usando a biblioteca `reqwest`.
//!
//! Ele possui duas missões muito importantes:
//! 1. `send_claim`: enviar a requisição de aperto de mão usando a nossa chave secreta de handshake embutida.
//! 2. `send_heartbeat`: enviar os nossos batimentos periódicos junto com os cabeçalhos criptográficos de assinatura Ed25519.
//!
//! Vamos ver como o carteiro trabalha?

use reqwest::StatusCode;
use thiserror::Error;

use crate::types::{
    ClaimRequest, ClaimResponse, HeartbeatRequest, HeartbeatResponse, SignatureHeaders,
};

const CLAIM_ENDPOINT: &str = "/api/tropic-of-cancer/claim";
const HEARTBEAT_ENDPOINT: &str = "/api/tropic-of-cancer/heartbeat";
/// Proteção contra pacotes gigantescos! O servidor limita as requisições a 64 KB.
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const AGENT_VERSION_HEADER: &str = "X-Mercator-Agent-Version";

/// Patinho, estes são os problemas que o nosso carteiro pode enfrentar durante as entregas!
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("failed to serialize request payload: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("payload is too large: {0} bytes")]
    PayloadTooLarge(usize),
    #[error("Mercator API returned ok=false")]
    NotOk,
    #[error("Mercator API returned {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
}

/// Envia o claim (aperto de mão inicial) ao servidor.
///
/// **Como funciona, patinho?**
/// 1. Nós convertemos o `ClaimRequest` em bytes usando a biblioteca JSON.
/// 2. Conferimos se o pacote não ultrapassa os 64 KB de segurança.
/// 3. Fazemos um `POST` para o endpoint `/api/tropic-of-cancer/claim`.
/// 4. Usamos um token estático especial no cabeçalho: `Authorization: Bearer <HANDSHAKE_KEY>`
///    para provar que o nosso instalador tem autoridade!
/// 5. Se o servidor disser "ok: true", retornamos o `ClaimResponse` contendo o nosso ID definitivo!
pub async fn send_claim(
    server_url: &str,
    handshake_key: &str,
    payload: &ClaimRequest,
) -> Result<ClaimResponse, ApiError> {
    let body = serde_json::to_vec(payload)?;
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(ApiError::PayloadTooLarge(body.len()));
    }

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint(server_url, CLAIM_ENDPOINT))
        .bearer_auth(handshake_key)
        .header(AGENT_VERSION_HEADER, &payload.agent.version)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await?;

    let response = parse_json_response::<ClaimResponse>(response).await?;
    if response.ok {
        Ok(response)
    } else {
        Err(ApiError::NotOk)
    }
}

/// Envia o heartbeat assinado digitalmente ao servidor.
///
/// **Esta é a parte mais segura de todas!**
/// Além de mandar o JSON com todo o inventário do computador, nós passamos 5 cabeçalhos especiais
/// que formam a nossa identidade criptográfica:
/// - `X-Mercator-Device-Id`: quem somos.
/// - `X-Mercator-Key-Id`: qual a nossa chave.
/// - `X-Mercator-Timestamp`: quando assinamos.
/// - `X-Mercator-Nonce`: identificador único aleatório contra ataques de repetição.
/// - `X-Mercator-Signature`: a assinatura matemática da String Canônica gerada com nossa chave privada!
///
/// O servidor usará a chave pública que cadastramos lá no claim para certificar que este pacote realmente
/// partiu de nós e que nenhum bit foi alterado na jornada!
pub async fn send_heartbeat(
    server_url: &str,
    payload: &HeartbeatRequest,
    sig: &SignatureHeaders,
) -> Result<HeartbeatResponse, ApiError> {
    let body = serde_json::to_vec(payload)?;
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(ApiError::PayloadTooLarge(body.len()));
    }

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint(server_url, HEARTBEAT_ENDPOINT))
        .header(AGENT_VERSION_HEADER, &payload.agent.version)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("X-Mercator-Device-Id", &sig.device_id)
        .header("X-Mercator-Key-Id", &sig.device_key_id)
        .header("X-Mercator-Timestamp", &sig.timestamp)
        .header("X-Mercator-Nonce", &sig.nonce)
        .header("X-Mercator-Signature", &sig.signature)
        .body(body)
        .send()
        .await?;

    let response = parse_json_response::<HeartbeatResponse>(response).await?;
    if response.ok {
        Ok(response)
    } else {
        Err(ApiError::NotOk)
    }
}

/// Um pequeno ajudante que junta a URL do servidor ao caminho do endpoint,
/// garantindo que não deixemos barras duplicadas no caminho (ex: `https://servidor.com//api/claim`).
fn endpoint(server_url: &str, path: &str) -> String {
    format!("{}{}", server_url.trim_end_matches('/'), path)
}

/// Converte o JSON retornado pelo servidor em um tipo Rust que possamos entender!
async fn parse_json_response<T>(response: reqwest::Response) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let response = ensure_success(response).await?;
    Ok(response.json::<T>().await?)
}

/// Confere se a resposta HTTP veio com status de sucesso (200 OK ou similar).
/// Se o servidor retornar um erro (ex: 400 Bad Request, 500 Server Error), nós capturamos o corpo
/// da mensagem e o limitamos a 512 caracteres para não poluir nossos arquivos de log!
async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read response body>".to_string());
    Err(ApiError::HttpStatus {
        status,
        body: truncate_body(&body),
    })
}

/// Corta o texto da resposta para no máximo 512 caracteres de segurança.
fn truncate_body(body: &str) -> String {
    const MAX_BODY_CHARS: usize = 512;
    body.chars().take(MAX_BODY_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_should_join_without_double_slash() {
        // Garantimos que a junção da URL funciona perfeitamente mesmo se a URL vier com barra final!
        assert_eq!(
            endpoint("https://mercator.example.com/", CLAIM_ENDPOINT),
            "https://mercator.example.com/api/tropic-of-cancer/claim"
        );
        assert_eq!(
            endpoint("https://mercator.example.com/", HEARTBEAT_ENDPOINT),
            "https://mercator.example.com/api/tropic-of-cancer/heartbeat"
        );
    }

    #[test]
    fn endpoint_should_handle_no_trailing_slash() {
        // E também se a URL vier limpa sem barra final!
        assert_eq!(
            endpoint("https://mercator.example.com", HEARTBEAT_ENDPOINT),
            "https://mercator.example.com/api/tropic-of-cancer/heartbeat"
        );
    }
}

