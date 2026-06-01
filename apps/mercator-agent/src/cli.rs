//! Olá, patinho! Este é o `cli.rs`.
//! Aqui é onde construímos a nossa interface de linha de comando (CLI).
//!
//! Usamos uma biblioteca maravilhosa chamada `clap` que lê o que o usuário digita no terminal
//! e converte automaticamente em structs e enums do Rust para que possamos tomar decisões!
//! Pense nisso como o menu de opções do nosso agente.

use clap::{Parser, Subcommand};

/// Patinho, este struct é a porta de entrada da nossa CLI.
/// Quando o usuário roda `mercator-agent <comando>`, o clap preenche este struct.
#[derive(Debug, Parser)]
#[command(name = "mercator-agent")]
#[command(version)]
#[command(about = "Mercator device inventory and presence agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Estes são os comandos disponíveis no nosso menu!
/// Cada variante representa uma ação diferente que o agente pode executar.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Rodar o agente em primeiro plano.
    /// Isso inicia um loop infinito que envia informações (heartbeats) periodicamente!
    Run,

    /// Ponto de entrada oculto especial usado pelo Gerenciador de Serviços do Windows (SCM)
    /// para rodar o nosso agente silenciosamente como um serviço do sistema operacional.
    #[command(name = "run-service", hide = true)]
    RunService,

    /// Instalar, desinstalar, iniciar ou parar o agente como Serviço do Windows.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },

    /// Configurar os dados iniciais do dispositivo.
    /// O instalador ou administrador roda esse comando informando o número de patrimônio da máquina,
    /// a matrícula do funcionário responsável e o código temporário de autorização!
    #[command(name = "configure")]
    Configure {
        /// Patrimônio do ativo (ex: TI-0234).
        #[arg(long)]
        asset_tag: String,
        /// Matrícula do funcionário responsável (ex: 12345).
        #[arg(long)]
        employee_registration: String,
        /// Código de autorização/provisionamento (ex: CAN-9F2K-8R).
        #[arg(long)]
        authorization_code: String,
    },

    /// Gerar o par de chaves Ed25519, enviar a chave pública para o servidor e registrar o dispositivo.
    /// Esse comando deve ser rodado logo após o `configure`. É o aperto de mão inicial!
    Claim,

    /// Coletar todos os dados de inventário (rede, bateria, sistema) e enviar um único heartbeat assinado agora mesmo!
    /// Excelente para testes manuais.
    HeartbeatOnce,

    /// Exibe o status atual e a configuração do agente na tela (em JSON bonito e seguro).
    Status,
}

/// E estes são os comandos específicos para gerenciar o Serviço do Windows.
#[derive(Debug, Subcommand)]
pub enum ServiceCommand {
    /// Instala o serviço no Windows para iniciar automaticamente com o computador.
    Install,
    /// Remove o serviço do Windows sem apagar os arquivos locais de configuração.
    Uninstall,
    /// Inicia o serviço que já está instalado.
    Start,
    /// Para o serviço que está em execução.
    Stop,
}

