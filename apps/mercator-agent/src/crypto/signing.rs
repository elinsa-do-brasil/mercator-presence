//! Olá, patinho! Este é o `signing.rs`.
//! Aqui é onde fazemos toda a mágica matemática de assinaturas usando o algoritmo Ed25519.
//!
//! Pense nisso como criar uma caneta esferográfica digital que só este dispositivo possui (a chave privada).
//! Qualquer pessoa no mundo pode pegar um pedaço de papel assinado por essa caneta e, usando uma lupa especial
//! pública (a chave pública), confirmar com 100% de certeza que a assinatura veio exatamente deste computador
//! e que ninguém alterou uma única letra do papel depois de assinado!
//!
//! Vamos ver como cada parte funciona?

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use sha2::{Digest, Sha256};

/// Patinho, esta é a nossa caixinha que guarda o par de chaves:
/// - `signing_key` (chave privada/assinadora): guarda o segredo de 32 bytes usado para assinar.
/// - `verifying_key` (chave pública/verificadora): enviada ao servidor para que ele possa conferir nossas assinaturas.
pub struct DeviceSigningKeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

/// Cria um par novinho em folha de chaves criptográficas Ed25519!
/// Usamos o `OsRng` (gerador de números aleatórios seguro do sistema operacional) para garantir
/// que ninguém consiga adivinhar a nossa chave privada. É como tirar uma sequência ultra aleatória de dados!
pub fn generate_keypair() -> DeviceSigningKeyPair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    DeviceSigningKeyPair {
        signing_key,
        verifying_key,
    }
}

/// Transforma a chave pública de verificação em uma string bonita do tipo "base64url" sem preenchimento (=).
/// Fazemos isso porque chaves criptográficas são dados binários brutos (uma sequência de bytes estranhos),
/// e strings em Base64URL são fáceis de enviar em JSON e cabeçalhos HTTP sem quebrar nada!
pub fn public_key_base64url(verifying_key: &VerifyingKey) -> String {
    URL_SAFE_NO_PAD.encode(verifying_key.as_bytes())
}

/// Salva a nossa chave privada preciosa em um arquivo no disco.
/// Como este é o nosso MVP (Mínimo Produto Viável), nós salvamos em formato binário simples no arquivo,
/// confiando que o sistema de arquivos do computador (com permissões ACLs de administrador) vai mantê-lo seguro.
/// Em fases futuras, patinho, usaremos a API de criptografia do Windows (DPAPI) para trancar essa chave a sete chaves!
pub fn save_private_key(signing_key: &SigningKey, path: &Path) -> Result<()> {
    // Primeiro, se as pastas do caminho até o arquivo não existirem, nós as criamos!
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Patinho, falhei ao criar a pasta da chave {}", parent.display()))?;
    }
    // Agora gravamos os 32 bytes da chave privada diretamente no arquivo!
    fs::write(path, signing_key.to_bytes())
        .with_context(|| format!("Patinho, falhei ao gravar a chave privada em {}", path.display()))?;
    Ok(())
}

/// Carrega a chave privada a partir do arquivo no disco.
/// Se o arquivo não existir ou não tiver exatamente 32 bytes, nós levantamos um erro bem explicativo.
pub fn load_private_key(path: &Path) -> Result<SigningKey> {
    let bytes = fs::read(path)
        .with_context(|| format!("Patinho, falhei ao ler o arquivo de chave privada em {}", path.display()))?;
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Patinho, o arquivo de chave privada não tem o tamanho esperado de 32 bytes!"))?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Calcula o "resumo" matemático (SHA-256) de um bloco de dados e retorna em Base64URL.
/// É como tirar a impressão digital de um documento. Se mudarmos um único bit no documento original,
/// a impressão digital muda completamente! Usamos isso para garantir que o corpo do heartbeat (JSON)
/// não foi adulterado no meio do caminho pela rede.
pub fn sha256_base64url(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    URL_SAFE_NO_PAD.encode(hash)
}

/// Cria a famosa "String Canônica".
/// Patinho, este conceito é muito importante! Criptografar coisas na rede é perigoso porque
/// pequenas variações de espaços ou quebras de linha podem estragar a assinatura.
/// Para evitar isso, nós juntamos todas as informações essenciais da requisição HTTP em um formato
/// estrito e padronizado por quebras de linha (`\n`).
///
/// O formato é:
/// ```text
/// MERCATOR-V1
/// POST
/// /api/tropic/heartbeat
/// {device_id}
/// {device_key_id}
/// {timestamp}
/// {nonce}
/// {body_sha256_base64url}
/// ```
/// Ao assinar exatamente essa estrutura, garantimos que ninguém consiga mudar a rota,
/// o ID do dispositivo, o timestamp ou o corpo da mensagem sem invalidar a assinatura!
pub fn build_canonical_string(
    device_id: &str,
    device_key_id: &str,
    timestamp: &str,
    nonce: &str,
    body_sha256_base64url: &str,
) -> String {
    format!(
        "MERCATOR-V1\nPOST\n/api/tropic/heartbeat\n{device_id}\n{device_key_id}\n{timestamp}\n{nonce}\n{body_sha256_base64url}"
    )
}

/// Assina a string canônica usando a nossa chave privada.
/// O resultado da assinatura é um bloco binário de 64 bytes, que também convertemos em Base64URL
/// para que viaje com segurança como um cabeçalho HTTP!
pub fn sign_canonical(canonical: &str, signing_key: &SigningKey) -> String {
    let signature = signing_key.sign(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(signature.to_bytes())
}

/// Gera um número aleatório único universal (UUID v4) para ser usado como Nonce (número usado uma única vez).
/// Isso serve para evitar "ataques de repetição", onde um invasor captura o nosso pacote assinado na rede e
/// tenta enviá-lo de novo para fingir que o agente está ativo. Com um Nonce único a cada envio, o servidor
/// rejeitará qualquer tentativa de repetir uma mensagem que já foi entregue!
pub fn generate_nonce() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_round_trip_should_preserve_key_material() {
        // Patinho, este teste simula o ciclo de vida completo da chave privada:
        // Criar, salvar em uma pasta temporária, recarregar e conferir se os bytes são idênticos!
        let dir =
            std::env::temp_dir().join(format!("mercator-signing-test-{}", std::process::id()));
        let path = dir.join("test-key.bin");

        let keypair = generate_keypair();
        save_private_key(&keypair.signing_key, &path).unwrap();
        let loaded = load_private_key(&path).unwrap();

        assert_eq!(keypair.signing_key.to_bytes(), loaded.to_bytes());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sign_and_verify_canonical_string() {
        // Testamos se a assinatura gerada por nós pode ser validada usando a nossa chave pública!
        let keypair = generate_keypair();
        let canonical = build_canonical_string(
            "dev_test",
            "devkey_test",
            "2026-06-01T12:00:00.000Z",
            "test-nonce-uuid",
            "abc123hash",
        );

        let signature_b64 = sign_canonical(&canonical, &keypair.signing_key);

        // Para verificar, decodificamos a assinatura do Base64URL de volta para bytes brutos.
        let sig_bytes = URL_SAFE_NO_PAD.decode(&signature_b64).unwrap();
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().unwrap());
        
        // E usamos a chave pública para validar de forma estrita se o texto confere com a assinatura.
        assert!(keypair
            .verifying_key
            .verify_strict(canonical.as_bytes(), &signature)
            .is_ok());
    }

    #[test]
    fn canonical_string_should_follow_spec_format() {
        // Garante que o formato da String Canônica tem exatamente 8 linhas separadas
        // e segue rigorosamente a especificação do protocolo do Mercator.
        let canonical = build_canonical_string(
            "dev_01J",
            "devkey_01J",
            "2026-06-01T12:15:00.000Z",
            "4f6ca7a0-5d8b-4b4d-b618-f8a9a0e8d9cc",
            "abcdef123456",
        );

        let lines: Vec<&str> = canonical.lines().collect();
        assert_eq!(lines.len(), 8);
        assert_eq!(lines[0], "MERCATOR-V1");
        assert_eq!(lines[1], "POST");
        assert_eq!(lines[2], "/api/tropic/heartbeat");
        assert_eq!(lines[3], "dev_01J");
        assert_eq!(lines[4], "devkey_01J");
    }

    #[test]
    fn sha256_base64url_should_be_deterministic() {
        // A função hash precisa ser determinística, ou seja, dar sempre o mesmo resultado
        // para a mesma entrada!
        let result = sha256_base64url(b"hello world");
        assert_eq!(result, sha256_base64url(b"hello world"));
        assert!(!result.is_empty());
    }

    #[test]
    fn generate_nonce_should_produce_unique_values() {
        // E os nonces gerados precisam ser completamente aleatórios e diferentes um do outro!
        let a = generate_nonce();
        let b = generate_nonce();
        assert_ne!(a, b);
    }
}

