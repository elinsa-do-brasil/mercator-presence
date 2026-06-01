mod api;
mod app;
mod cli;
mod collector;
mod config;
mod crypto;
mod embedded;
mod logging;
mod security;
mod service;
mod types;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};

/// Olá, patinho! Este é o `main.rs`, o ponto de partida absoluto do Mercator Agent!
/// Quando o sistema operacional inicia o nosso executável, a execução começa exatamente aqui!
///
/// **O que fazemos no início da jornada?**
/// 1. Nós pedimos para o `clap` interpretar todos os argumentos passados via terminal (`Cli::parse()`).
/// 2. Inicializamos o nosso logger (`logging::init()`) para registrar tudo que acontece em um arquivo rotativo de log.
/// 3. Fazemos um `match` para ver qual comando o usuário escolheu e direcionamos para a função correta!
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // O logger registra eventos importantes (como falhas ou sucessos de batimentos) em disco.
    let _log_guard = logging::init()?;

    match cli.command {
        // 1. Rodar o loop eterno de heartbeats em primeiro plano!
        Command::Run => app::run_foreground().await,
        
        // 2. Comando interno usado pelo Windows Service Manager para despachar o serviço.
        Command::RunService => service::run_dispatcher(),
        
        // 3. Salvar o patrimônio e matrícula no HD local da máquina.
        Command::Configure {
            asset_tag,
            employee_registration,
            authorization_code,
        } => app::configure(&asset_tag, &employee_registration, &authorization_code),
        
        // 4. Executar o claim (registro e aperto de mão inicial).
        Command::Claim => {
            // Patinho, o código de provisionamento/autorização temporário é exigido pelo claim.
            // Para não salvar esse código sensível em disco (o que seria inseguro!), nós pedimos
            // que ele seja passado através de uma variável de ambiente temporária durante a instalação!
            let code = std::env::var("MERCATOR_AUTHORIZATION_CODE")
                .unwrap_or_default();
            if code.is_empty() {
                anyhow::bail!(
                    "authorization code not found. Set MERCATOR_AUTHORIZATION_CODE env var \
                     or use the installer pipeline."
                );
            }
            app::claim(&code).await
        }
        
        // 5. Coletar o inventário completo e disparar um único batimento cardíaco assinado agora mesmo!
        Command::HeartbeatOnce => app::heartbeat_once().await,
        
        // 6. Gerenciar a instalação/remoção/status do Windows Service.
        Command::Service { command } => service::handle_command(command),
        
        // 7. Mostrar o status atual do agente (se está registrado, IDs, caminhos de arquivo, etc.).
        Command::Status => app::status(),
    }
}

