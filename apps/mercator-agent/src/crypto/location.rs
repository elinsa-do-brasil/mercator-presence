//! Olá, patinho! Este é o `location.rs`.
//! Aqui é onde vai morar a criptografia super segura da geolocalização do dispositivo!
//!
//! Por que precisamos disso?
//! Geolocalização é um dado muito sensível de privacidade. Se alguém interceptar a rede ou se
//! tivermos um vazamento de logs no servidor intermediário, não queremos expor onde o funcionário
//! está trabalhando!
//! Para resolver isso, usamos criptografia assimétrica ponta a ponta (como HPKE ou sealed box do Libsodium).
//! O agente criptografa as coordenadas usando uma chave pública embutida, e *apenas* a chave privada
//! correspondente (que fica ultra guardada e isolada dentro do backend seguro do Mercator) consegue descriptografar.
//!
//! **Nota de Desenvolvimento:**
//! No MVP (Fase 3), nós apenas preparamos toda a fiação elétrica e os tipos de dados para que os heartbeats
//! consigam trafegar. A implementação matemática real da criptografiaHPKE será feita na Fase 4!
//! Então, por enquanto, este arquivo funciona como um belo "esqueleto" (stub).

use serde::{Deserialize, Serialize};

/// Patinho, este struct representa os dados da localização em texto plano (aberto).
/// São as coordenadas brutas antes de serem trancadas pelo cofre criptográfico!
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationPlaintext {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: Option<f64>,
    pub source: String,
    pub collected_at: String,
}

/// E este struct representa o pacote criptografado final que viaja na rede.
/// Note que ele não mostra latitude nem longitude! Ele mostra apenas um bloco de caracteres indecifráveis (`ciphertext`),
/// o identificador da chave pública usada (`key_id`), e detalhes técnicos sobre o algoritmo!
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedLocationPayload {
    pub encrypted: bool,
    pub key_id: String,
    pub algorithm: String,
    pub ciphertext: String,
    pub encapsulated_key: String,
    pub nonce: Option<String>,
}

/// O nosso "Trancador de Localização".
/// Ele precisa saber qual o ID da chave do servidor e ter os bytes brutos da chave pública
/// para conseguir trancar o baú!
#[allow(dead_code)]
pub struct LocationEncryptor {
    pub key_id: String,
    pub algorithm: String,
    #[allow(dead_code)]
    pub public_key_bytes: Vec<u8>,
}

#[allow(dead_code)]
impl LocationEncryptor {
    /// Tenta construir o nosso trancador usando as chaves públicas embutidas (que vieram lá do `.env`).
    /// Se o usuário compilar o agente sem passar as variáveis de chave de localização, nós retornamos `None`,
    /// o que significa que o agente não fará a criptografia de geolocalização.
    pub fn from_embedded() -> Option<Self> {
        let key_id = crate::embedded::LOCATION_PUBLIC_KEY_ID;
        let public_key_b64 = crate::embedded::LOCATION_PUBLIC_KEY;

        // Se estiver em branco, significa que a funcionalidade está desativada na compilação.
        if key_id.is_empty() || public_key_b64.is_empty() {
            return None;
        }

        // Decodificamos a chave pública de Base64 padrão para bytes brutos.
        let public_key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            public_key_b64,
        )
        .ok()?;

        Some(Self {
            key_id: key_id.to_string(),
            algorithm: "HPKE-X25519-HKDF-SHA256-AES256GCM".to_string(),
            public_key_bytes,
        })
    }

    /// Tranca os dados de localização!
    ///
    /// # Esqueleto (Stub)
    /// Patinho, por enquanto esta função apenas emite um aviso no log e retorna `None`
    /// porque a criptografia HPKE real será construída na próxima fase. Mas os tipos
    /// e a lógica do fluxo principal já estão prontos para receber o retorno!
    #[allow(unused_variables)]
    pub fn encrypt(&self, plaintext: &LocationPlaintext) -> Option<EncryptedLocationPayload> {
        // TODO(fase-4): implementar criptografia real com HPKE ou sealed box.
        tracing::warn!("location encryption is not yet implemented (stub)");
        None
    }
}

