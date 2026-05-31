# Mercator Agent

Mercator Agent is the Windows-focused Rust agent for Mercator device inventory and presence. The MVP enrolls a device, stores its device credentials locally, sends heartbeats, and prepares contracts for future internal messages without adding UI to the service.

## What It Collects

- Hostname and current user.
- Manufacturer, model, serial number when the OS exposes them.
- OS name/version, CPU brand, memory usage, and uptime.
- Local private IPs and MAC addresses.
- Battery fields as nullable placeholders for the MVP.
- Agent version and collection timestamp.

## What It Does Not Collect

- User files, documents, photos, browser history, passwords, clipboard, keystrokes, screenshots, open windows, detailed process lists, precise geolocation, packet contents, remote shell, remote commands, or arbitrary download/execute behavior.

## Build

```powershell
cargo build --release -p mercator-agent
```

From this directory:

```powershell
cargo build --release
```

## Foreground Run

```powershell
.\target\release\mercator-agent.exe run
```

Foreground mode loads config, sends a heartbeat immediately, and continues the heartbeat loop.

## Enrollment

```powershell
.\target\release\mercator-agent.exe enroll --server-url "https://mercator.exemplo.com.br" --token "ENROLL_TOKEN"
```

The enrollment token is sent once and is never saved. The returned `deviceId`, `deviceToken`, and optional heartbeat interval are saved to config.

## Test Heartbeat

```powershell
.\target\release\mercator-agent.exe heartbeat-once
```

This prints hostname, serial, current user, local private IPs, and send status. Tokens are not printed.

## Config

Windows path:

```text
C:\ProgramData\Mercator\Agent\config.json
```

Non-Windows development uses a safe user config directory, or `MERCATOR_AGENT_HOME` when set.

Example:

```json
{
  "serverUrl": "https://mercator.exemplo.com.br",
  "deviceId": "dev_xxx",
  "deviceToken": "token_emitido_pelo_servidor",
  "heartbeatIntervalSeconds": 900,
  "agentVersion": "0.1.0"
}
```

Show masked config:

```powershell
.\target\release\mercator-agent.exe config show
```

## Logs

Windows path:

```text
C:\ProgramData\Mercator\Agent\logs\agent.log
```

Logs must not contain full tokens. Network/API errors are summarized.

## Windows Service

Run these commands from an elevated PowerShell:

```powershell
.\target\release\mercator-agent.exe service install
.\target\release\mercator-agent.exe service start
.\target\release\mercator-agent.exe service stop
.\target\release\mercator-agent.exe service uninstall
```

Service metadata:

- Service name: `MercatorAgent`
- Display name: `Mercator Agent`
- Description: `Mercator device inventory and presence agent`

Uninstall does not delete config.

## Endpoints

Enrollment:

```http
POST /api/agent/enroll
Authorization: Bearer <ENROLLMENT_TOKEN>
Content-Type: application/json
```

Heartbeat:

```http
POST /api/agent/heartbeat
Authorization: Bearer <DEVICE_TOKEN>
Content-Type: application/json
```

Future message contracts, not implemented in the service UI:

```http
GET /api/agent/messages/poll
POST /api/agent/messages/:id/ack
```

## Development Checks

```bash
cargo fmt
cargo test -p mercator-agent
cargo check -p mercator-agent
```

## Next Steps

1. Auto-update with signed binaries.
2. MSI installer.
3. Separate notifier for HTML messages using Tauri/WebView2.
4. `/api/agent/messages/poll`.
5. `/api/agent/messages/:id/ack`.
6. Backend location inference from known networks.
7. Mercator presence map.
8. Presence event retention policies.
9. Binary signing/codesign.
10. Build and release via GitHub Actions.
