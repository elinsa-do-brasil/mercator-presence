# Prompt para o Codex — Mercator Agent MVP

> Objetivo: criar o primeiro MVP do **Mercator Agent**, um agent corporativo em Rust para inventário, presença/last seen e base futura para mensagens internas.

## Prompt para colar no Codex

Você está trabalhando no projeto **Mercator Presence / Mercator Agent**, um sistema corporativo de inventário e presença de dispositivos usado junto ao Mercator.

Quero criar o MVP de um agent em Rust para Windows, com foco em:

1. Instalar e rodar como **Windows Service**.
2. Fazer **enrollment** do dispositivo via API.
3. Enviar **heartbeat periódico**.
4. Coletar inventário básico do computador.
5. Coletar informações básicas da rede atual para “último visto”.
6. Preparar a estrutura para futuras mensagens HTML/notificações, mas **sem implementar UI agora**.
7. Seguir segurança, privacidade e manutenção decentes desde o começo.

---

## Contexto do workspace

Antes de codar, faça uma inspeção real do repositório.

Verifique:

- Estrutura atual do projeto.
- Se existe monorepo.
- Se existe app Next.js/API.
- Se existe pasta `apps`, `packages`, `agents`, `src`, `server`, `api`, `db` etc.
- Se existe configuração de MCP.
- Se existem skills instaladas em `.agents/skills`.
- Leia `.agents/skills/README.md`, se existir.
- Leia `.agents/skills-lock.json`, se existir.
- Use as skills de Rust instaladas quando forem relevantes:
  - `rust-best-practices`
  - `rust-async-patterns`
  - `rust-patterns`
  - `rust-skills`
  - `rust-testing`

Se houver MCP disponível para documentação atualizada, use-o para consultar docs atuais de crates e APIs antes de decidir detalhes de implementação. Não invente API de crate no chute, porque Rust não perdoa e o compilador já humilha o suficiente.

Se o projeto tiver padrão claro de organização, siga o padrão existente.

Se não houver padrão claro, crie o agent em:

~~~txt
agents/mercator-agent
~~~

---

## Stack desejada

Use Rust estável.

Crates sugeridos:

~~~toml
windows-service = "0.8"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sysinfo = "0.39"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "2"
time = { version = "0.3", features = ["serde", "formatting"] }
directories = "6"
~~~

Ajuste versões se necessário, mas explique o motivo.

Use:

- `windows-service` para integração com Windows Service.
- `tokio` para runtime assíncrono.
- `reqwest` para HTTPS/JSON.
- `serde`/`serde_json` para payloads.
- `sysinfo` para dados básicos de sistema.
- `tracing` para logs.
- `clap` para CLI.
- `anyhow`/`thiserror` para erros.

Evite dependências desnecessárias.

---

## Escopo do MVP

Criar um binário:

~~~txt
mercator-agent.exe
~~~

Com comandos:

~~~bash
mercator-agent run
mercator-agent service install
mercator-agent service uninstall
mercator-agent service start
mercator-agent service stop
mercator-agent enroll --server-url <URL> --token <TOKEN>
mercator-agent heartbeat-once
mercator-agent config show
~~~

### Comportamento esperado

`run`

- Roda em foreground.
- Útil para desenvolvimento/debug.
- Carrega config.
- Envia heartbeat ao iniciar.
- Entra no loop de heartbeat.

`service install`

- Instala o agent como Windows Service.
- Nome do serviço: `MercatorAgent`
- Display name: `Mercator Agent`
- Description: `Mercator device inventory and presence agent`

`service uninstall`

- Remove o serviço.
- Não deve apagar config automaticamente.

`service start`

- Inicia o serviço.

`service stop`

- Para o serviço.

`enroll`

- Registra o dispositivo na API do Mercator usando enrollment token.
- Salva `deviceId` e `deviceToken` retornados.
- Nunca salva o enrollment token.

`heartbeat-once`

- Coleta dados.
- Envia uma vez.
- Imprime resumo no terminal:
  - hostname
  - serial
  - usuário atual
  - IPs locais
  - status do envio

`config show`

- Mostra config atual mascarando tokens.
- Nunca imprimir token completo.

---

## Paths locais

Criar diretório:

~~~txt
C:\ProgramData\Mercator\Agent\
~~~

Criar arquivo:

~~~txt
C:\ProgramData\Mercator\Agent\config.json
~~~

Criar logs em:

~~~txt
C:\ProgramData\Mercator\Agent\logs\agent.log
~~~

Se o código estiver rodando fora do Windows, usar paths seguros para desenvolvimento, sem quebrar build em Linux/macOS. O foco do serviço é Windows, mas o projeto deve pelo menos compilar o máximo possível ou isolar código Windows com `cfg(windows)`.

---

## Formato do config

Use algo próximo disso:

~~~json
{
  "serverUrl": "https://mercator.exemplo.com.br",
  "deviceId": "dev_xxx",
  "deviceToken": "token_emitido_pelo_servidor",
  "heartbeatIntervalSeconds": 900,
  "agentVersion": "0.1.0"
}
~~~

Regras:

- `serverUrl` não pode ser vazio.
- `deviceId` não pode ser vazio depois do enroll.
- `deviceToken` não pode ser vazio depois do enroll.
- `heartbeatIntervalSeconds` padrão: `900`.
- Mas permitir o servidor sobrescrever no enroll.
- Mascare tokens em logs e comandos.

---

## Segurança e privacidade

Este agent NÃO deve se comportar como spyware.

Regras obrigatórias:

- Nunca coletar arquivos do usuário.
- Nunca coletar histórico de navegador.
- Nunca capturar tela.
- Nunca registrar teclas.
- Nunca coletar conteúdo de clipboard.
- Nunca executar comando remoto.
- Nunca abrir porta local para controle remoto.
- Nunca implementar shell remoto.
- Nunca implementar download/execução arbitrária de comandos.
- Nunca rodar oculto de forma deliberada.
- O serviço deve ser visível e desinstalável.
- Comunicação sempre outbound HTTPS para o servidor.
- Logs não devem conter tokens completos.
- Enrollment token nunca deve ser salvo.
- Device token pode ser salvo no config, mas nunca impresso integralmente.
- Se algum dado for opcional ou sensível, prefira não coletar no MVP.

Não implementar geolocalização precisa neste MVP.

A localização inicial deve ser inferida no backend com base em:

- IP público visto pelo servidor.
- rede conhecida.
- gateway.
- faixa de IP.
- SSID, futuramente se seguro.
- base/localidade cadastrada.

---

## Endpoints esperados

Se o backend ainda não existir no repositório, crie apenas contratos/types, exemplos e TODOs. Não quebrar o app existente.

### 1. Enrollment

Request:

~~~http
POST /api/agent/enroll
Authorization: Bearer <ENROLLMENT_TOKEN>
Content-Type: application/json
~~~

Payload:

~~~json
{
  "hostname": "NB-FISCAL-023",
  "serialNumber": "ABC123",
  "manufacturer": "Dell Inc.",
  "model": "Latitude 3420",
  "osName": "Windows",
  "osVersion": "10.0.19045",
  "agentVersion": "0.1.0",
  "collectedAt": "2026-05-31T03:00:00Z"
}
~~~

Resposta esperada:

~~~json
{
  "deviceId": "dev_xxx",
  "deviceToken": "secret_device_token",
  "heartbeatIntervalSeconds": 900
}
~~~

### 2. Heartbeat

Request:

~~~http
POST /api/agent/heartbeat
Authorization: Bearer <DEVICE_TOKEN>
Content-Type: application/json
~~~

Payload:

~~~json
{
  "deviceId": "dev_xxx",
  "hostname": "NB-FISCAL-023",
  "currentUser": "ELINSA\\joao.silva",
  "agentVersion": "0.1.0",
  "occurredAt": "2026-05-31T03:15:00Z",
  "system": {
    "manufacturer": "Dell Inc.",
    "model": "Latitude 3420",
    "serialNumber": "ABC123",
    "osName": "Windows",
    "osVersion": "10.0.19045",
    "cpuBrand": "Intel(R) Core(TM) i5",
    "totalMemoryBytes": 17179869184,
    "usedMemoryBytes": 8589934592,
    "uptimeSeconds": 123456
  },
  "network": {
    "networkType": "wifi_or_ethernet_or_unknown",
    "privateIps": ["192.168.10.43"],
    "macAddresses": ["00:11:22:33:44:55"],
    "gatewayIp": null,
    "ssid": null,
    "bssidHash": null,
    "publicIpSeenByServer": null
  },
  "battery": {
    "percent": null,
    "isCharging": null
  }
}
~~~

Observações:

- `publicIpSeenByServer` deve preferencialmente ser preenchido pelo backend a partir da requisição.
- SSID/BSSID podem ficar `null` no MVP.
- Se no futuro coletar BSSID, armazenar hash, não valor bruto.
- Não chamar API de localização do Windows neste MVP.

### 3. Mensagens futuras

Não implementar agora, mas preparar os tipos/estrutura para futura expansão:

~~~http
GET /api/agent/messages/poll
POST /api/agent/messages/:id/ack
~~~

A UI de mensagens deve ser um app separado no futuro, não dentro do Windows Service.

---

## Estrutura sugerida de módulos

Organize de forma limpa, por exemplo:

~~~txt
agents/mercator-agent/
  Cargo.toml
  README.md
  src/
    main.rs
    cli.rs
    config.rs
    api.rs
    collector/
      mod.rs
      system.rs
      network.rs
      user.rs
      battery.rs
    service/
      mod.rs
      windows.rs
    logging.rs
    security.rs
    types.rs
~~~

Pode ajustar conforme o padrão do repo.

### `collector`

Criar funções:

~~~rust
collect_system_info()
collect_network_info()
collect_current_user()
collect_battery_info()
collect_enroll_payload()
collect_heartbeat_payload()
~~~

### `api`

Criar funções:

~~~rust
enroll(server_url, enrollment_token, payload)
send_heartbeat(server_url, device_token, payload)
~~~

### `config`

Criar funções:

~~~rust
load_config()
save_config()
ensure_config_dir()
mask_secret()
validate_config()
~~~

### `service`

Criar funções para:

- install
- uninstall
- start
- stop
- run service loop
- responder ao stop signal do Windows Service Control Manager

---

## Loop do serviço

Ao iniciar:

1. Inicializar logging.
2. Carregar config.
3. Se não houver config/deviceToken:
   - logar erro claro.
   - parar serviço sem panic.
4. Enviar heartbeat ao iniciar.
5. Aguardar `heartbeatIntervalSeconds`.
6. Enviar novos heartbeats em loop.
7. Se houver erro de rede:
   - logar erro resumido.
   - usar backoff simples.
   - não travar o serviço.
   - não apagar config.
8. Responder corretamente ao stop do Windows Service.

Backoff simples sugerido:

~~~txt
1 min
2 min
5 min
10 min
máximo 15 min
~~~

---

## Dados coletados no MVP

Coletar apenas:

- hostname
- usuário atual
- fabricante
- modelo
- serial
- nome do sistema operacional
- versão/build do Windows
- CPU
- memória total
- memória usada
- uptime
- IPs locais
- MACs das interfaces
- gateway, se simples
- tipo de rede, se simples
- porcentagem de bateria, se simples
- se está carregando, se simples
- versão do agent
- horário da coleta

Não coletar:

- lista de arquivos
- documentos
- fotos
- prints
- histórico de navegador
- senhas
- clipboard
- processos detalhados
- janelas abertas
- teclas
- localização precisa
- conteúdo de rede
- comando remoto

---

## Backend/contratos

Se o backend Mercator estiver neste repo e for simples adicionar rotas/types sem grande mudança, implemente apenas o mínimo:

- types/DTOs para enroll e heartbeat.
- rota stub para desenvolvimento local, se fizer sentido.
- TODO claro para persistência real.

Se o backend não estiver claro, não invente uma arquitetura paralela. Documente os contratos no README do agent.

---

## Testes

Adicionar testes unitários onde fizer sentido:

- parse/validação de config.
- criação de paths.
- mascaramento de token.
- montagem de payload.
- serialização JSON.
- comportamento quando config está ausente.
- normalização de `serverUrl`.

Não precisa testar Windows Service profundamente agora, mas isole a lógica para facilitar teste.

---

## README obrigatório

Criar `README.md` dentro do agent com:

1. O que é o Mercator Agent.
2. O que ele coleta.
3. O que ele NÃO coleta.
4. Como compilar.
5. Como rodar em foreground.
6. Como fazer enroll.
7. Como enviar heartbeat de teste.
8. Como instalar como Windows Service.
9. Como iniciar/parar/remover serviço.
10. Endpoints esperados.
11. Estrutura do config.
12. Próximos passos.

Exemplos de comandos:

~~~powershell
cargo build --release
.\target\release\mercator-agent.exe enroll --server-url "https://mercator.exemplo.com.br" --token "ENROLL_TOKEN"
.\target\release\mercator-agent.exe heartbeat-once
.\target\release\mercator-agent.exe service install
.\target\release\mercator-agent.exe service start
.\target\release\mercator-agent.exe service stop
.\target\release\mercator-agent.exe service uninstall
~~~

---

## Próximos passos documentados, não implementados

Documentar como futuro:

1. Auto-update com binários assinados.
2. Instalador MSI.
3. Notifier separado para mensagens HTML usando Tauri/WebView2.
4. Endpoint `/api/agent/messages/poll`.
5. Endpoint `/api/agent/messages/:id/ack`.
6. Inferência de localização por redes conhecidas no backend.
7. Mapa de presença no Mercator.
8. Políticas de retenção de eventos de presença.
9. Assinatura de binário/codesign.
10. Build/release via GitHub Actions.

---

## Critérios de aceite

A tarefa estará pronta quando:

- O projeto Rust do agent existir e compilar.
- A CLI responder aos comandos definidos.
- `enroll` montar payload e chamar a API.
- `heartbeat-once` coletar dados e tentar enviar.
- O serviço Windows tiver implementação inicial isolada por `cfg(windows)`.
- Logs forem criados sem vazar tokens.
- Config for salva em local adequado.
- README estiver completo.
- Testes básicos passarem.
- O código não implementar spyware, shell remoto ou UI dentro do service.
- O relatório final listar arquivos criados/alterados e como testar no Windows.

---

## Observação final

Priorize o MVP funcionando com clareza. Não tente resolver auto-update, UI de mensagens, geolocalização precisa e instalador MSI nesta primeira entrega.

O objetivo agora é:

~~~txt
enroll → config salvo → heartbeat-once → service loop básico
~~~

O resto é fase 2. Não transforme o primeiro commit em um Frankenstein com crachá.
