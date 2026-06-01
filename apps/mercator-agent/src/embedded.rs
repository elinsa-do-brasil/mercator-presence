//! Olá, patinho! Este arquivo é o `embedded.rs`.
//! A função dele é segurar as constantes que o nosso `build.rs` preparou e embutiu
//! no binário durante a compilação.
//!
//! Pense nessas constantes como "configurações de fábrica" ou "DNA" do nosso agente.
//! Elas não mudam depois que o programa foi compilado. Se uma variável não for informada
//! no arquivo `.env` durante a compilação, o `build.rs` insere uma string vazia `""`.
//! Isso permite compilar em ambientes de teste sem segredos, mas se o agente precisar
//! de fato delas em execução, nossas funções de validação vão reclamar educadamente!

/// Patinho, esta é a URL do servidor Mercator para onde enviaremos todas as informações.
/// Ela é extraída direto de `MERCATOR_SERVER_URL` na compilação.
pub const SERVER_URL: &str = env!("MERCATOR_SERVER_URL");

/// Esta é a chave secreta de Handshake (aperto de mão).
/// Ela serve apenas para provar ao servidor que o nosso instalador tem autorização para
/// registrar este computador como um novo dispositivo (`claim`).
pub const HANDSHAKE_KEY: &str = env!("MERCATOR_HANDSHAKE_KEY");

/// Este é o identificador único da chave pública que usamos para criptografar
/// a geolocalização do computador, garantindo privacidade completa do usuário.
pub const LOCATION_PUBLIC_KEY_ID: &str = env!("MERCATOR_LOCATION_PUBLIC_KEY_ID");

/// E esta é a chave pública em si (codificada em Base64).
/// Nós a usaremos no futuro para trancar a localização do usuário com criptografia assimétrica,
/// de modo que apenas o servidor central do Mercator consiga abrir e ver a geolocalização!
#[allow(dead_code)]
pub const LOCATION_PUBLIC_KEY: &str = env!("MERCATOR_LOCATION_PUBLIC_KEY");

/// Patinho, esta função valida se as configurações mínimas para o funcionamento básico
/// do agente estão presentes.
/// Ela confere se a URL do servidor e a chave de handshake não estão em branco.
/// Se alguma estiver faltando, retorna um `Err` avisando que precisamos compilar com um `.env` válido.
pub fn validate_embedded_config() -> Result<(), String> {
    let mut missing = Vec::new();

    // Se a URL estiver vazia, adicionamos na lista de pendências
    if SERVER_URL.is_empty() {
        missing.push("MERCATOR_SERVER_URL");
    }
    // Se a chave de handshake estiver vazia, também adicionamos
    if HANDSHAKE_KEY.is_empty() {
        missing.push("MERCATOR_HANDSHAKE_KEY");
    }

    if missing.is_empty() {
        // Tudo certo, patinho! Podemos prosseguir!
        Ok(())
    } else {
        // Xi, patinho... Está faltando coisa essencial!
        Err(format!(
            "missing build-time variables: {}. Rebuild with a valid .env file.",
            missing.join(", ")
        ))
    }
}

/// Patinho, esta função serve para conferir se temos as chaves necessárias para
/// criptografar a localização.
/// Se o ID da chave ou a própria chave pública estiverem em branco, avisamos que
/// a funcionalidade de geolocalização segura não estará disponível.
#[allow(dead_code)]
pub fn validate_location_config() -> Result<(), String> {
    let mut missing = Vec::new();

    if LOCATION_PUBLIC_KEY_ID.is_empty() {
        missing.push("MERCATOR_LOCATION_PUBLIC_KEY_ID");
    }
    if LOCATION_PUBLIC_KEY.is_empty() {
        missing.push("MERCATOR_LOCATION_PUBLIC_KEY");
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "missing build-time location variables: {}. Location encryption is unavailable.",
            missing.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_constants_should_be_available() {
        // Patinho, este teste simples serve apenas para garantir que as constantes
        // existem, compilam e podem ser acessadas sem quebrar o programa.
        // Em builds de teste sem arquivo `.env`, elas serão strings vazias.
        let _ = SERVER_URL;
        let _ = HANDSHAKE_KEY;
        let _ = LOCATION_PUBLIC_KEY_ID;
        let _ = LOCATION_PUBLIC_KEY;
    }
}

