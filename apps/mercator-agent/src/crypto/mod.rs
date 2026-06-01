//! Olá, patinho! Este é o módulo de criptografia do Mercator Agent.
//! Pense nele como a nossa caixa de ferramentas de segurança. Aqui dentro, organizamos tudo que
//! lida com proteção matemática e assinaturas no agente.
//!
//! Ele possui duas divisões muito legais:
//! - `signing` (assinaturas): serve para gerarmos o par de chaves do dispositivo,
//!   assinar as requisições enviadas ao servidor para provar que somos nós mesmos (identidade),
//!   e evitar que alguém tente falsificar os heartbeats!
//! - `location` (localização): serve para criptografar as coordenadas geográficas
//!   do computador de forma que somente o servidor Mercator consiga ler.

pub mod location;
pub mod signing;

