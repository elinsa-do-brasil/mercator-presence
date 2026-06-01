# Mercator Agent

Mercator Agent e o agent Rust, com foco em Windows, para inventario e presenca de dispositivos no Mercator. O MVP coleta dados basicos do computador e envia heartbeats para a API de ingestao.

## O Que Ele Coleta

- Hostname e usuario atual.
- Fabricante, modelo e serial quando o sistema operacional expuser esses dados.
- Nome, versao/build do sistema operacional, CPU, memoria e uptime.
- IPs privados locais e MACs validos das interfaces.
- Campos de bateria como nulos no MVP.
- Versao do agent, plataforma e horario da coleta.

## O Que Ele Nao Coleta

Arquivos do usuario, documentos, fotos, historico de navegador, senhas, clipboard, teclas, prints, janelas abertas, lista detalhada de processos, geolocalizacao precisa, conteudo de rede, shell remoto, comandos remotos ou download/execucao arbitraria.

## Build

```powershell
cargo build --release -p mercator-agent
```

Dentro desta pasta:

```powershell
cargo build --release
```

## Configuracao

```powershell
.\target\release\mercator-agent.exe configure --server-url "https://mercator.exemplo.com.br" --api-key "tc_live_xxxxx_SECRET"
```

`enroll` continua aceito como alias de compatibilidade, mas o contrato atual usa `API_KEY`, nao enrollment token:

```powershell
.\target\release\mercator-agent.exe enroll --server-url "https://mercator.exemplo.com.br" --token "tc_live_xxxxx_SECRET"
```

A chave e salva no config local e nunca deve aparecer inteira em logs ou comandos de visualizacao.

## Heartbeat De Teste

```powershell
.\target\release\mercator-agent.exe heartbeat-once
```

O comando coleta os dados, envia uma chamada para `/api/tropic-of-cancer` e imprime hostname, serial, usuario atual, IPs privados, status do envio e `deviceId` retornado pelo servidor.

## Execucao Em Foreground

```powershell
.\target\release\mercator-agent.exe run
```

O modo foreground carrega o config, envia um heartbeat ao iniciar e continua no loop usando `heartbeatIntervalSeconds`.

## Config Local

Windows:

```text
C:\ProgramData\Mercator\Agent\config.json
```

Fora do Windows, o agent usa um diretorio seguro de desenvolvimento, ou `MERCATOR_AGENT_HOME` quando definido.

Exemplo:

```json
{
  "serverUrl": "https://mercator.exemplo.com.br",
  "apiKey": "tc_live_xxxxx_SECRET",
  "heartbeatIntervalSeconds": 900,
  "agentVersion": "0.1.0"
}
```

Visualizar com segredo mascarado:

```powershell
.\target\release\mercator-agent.exe config show
```

## Logs

Windows:

```text
C:\ProgramData\Mercator\Agent\logs\agent.log
```

Logs nao devem conter tokens completos. Erros de rede/API sao resumidos.

## Contrato Da API

Heartbeat:

```http
POST /api/tropic-of-cancer
Authorization: Bearer <API_KEY>
Content-Type: application/json
X-Mercator-Agent-Version: 0.1.0
```

O payload segue o contrato de ingestao com `kind: "heartbeat"` e blocos `device`, `user`, `agent`, `system`, `network`, `battery` e `collectedAt`. O agent valida que existe ao menos um identificador forte antes do envio: serial, MAC valido ou `hostname + manufacturer + model`.

O servidor deve responder:

```json
{
  "ok": true,
  "deviceId": "uuid-do-dispositivo",
  "receivedAt": "2026-05-31T12:00:00.000Z",
  "serverTime": "2026-05-31T12:00:00.100Z"
}
```

## Windows Service

Execute em um PowerShell elevado:

```powershell
.\target\release\mercator-agent.exe service install
.\target\release\mercator-agent.exe service start
.\target\release\mercator-agent.exe service stop
.\target\release\mercator-agent.exe service uninstall
```

Metadados do servico:

- Service name: `MercatorAgent`
- Display name: `Mercator Agent`
- Description: `Mercator device inventory and presence agent`

`service uninstall` nao apaga o config.

## Verificacoes De Desenvolvimento

```bash
cargo fmt
cargo test -p mercator-agent
cargo check -p mercator-agent
```

## Proximos Passos

1. Auto-update com binarios assinados.
2. Instalador MSI.
3. Notifier separado para mensagens HTML usando Tauri/WebView2.
4. Endpoints futuros de mensagens.
5. Inferencia de localizacao no backend por redes conhecidas.
6. Mapa de presenca no Mercator.
7. Politicas de retencao de eventos de presenca.
8. Assinatura de binario/codesign.
9. Build/release via GitHub Actions.
