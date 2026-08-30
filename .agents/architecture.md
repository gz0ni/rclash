# Architecture

## Entry Point
`src/main.rs` → `rclash::app::RClashApp` (eframe::App). Initializes `CoreManager`, `AppConfig`, `SysProxy` and runs `eframe::run_native`.

## Core Layer

### app
- responsibility: top-level egui app, tab routing, global state
- key files: `src/app.rs`, `src/main.rs`
- notable: `enum Tab { Dashboard, Profiles, Proxies, Connections, Logs, Settings }`, SidePanel navigation, CentralPanel content

### crates/rclash-config
- responsibility: mihomo YAML parsing, validation, profile persistence
- key files: `crates/rclash-config/src/lib.rs`, `crates/rclash-config/src/profile.rs`
- notable: `serde_yaml` models, stored in `dirs::config_dir()/RClash` (Win `%APPDATA%/RClash`, Linux `~/.config/rclash`, macOS `~/Library/Application Support/RClash`)

### crates/rclash-core-manager
- responsibility: spawn/manage `rclash-core` sidecar, REST client to mihomo API
- key files: `crates/rclash-core-manager/src/lib.rs`, `crates/rclash-core-manager/src/process.rs`, `crates/rclash-core-manager/src/api.rs`
- notable: `Command::new("rclash-core")`, healthcheck `GET http://127.0.0.1:9090/version`, traffic WS `/traffic`, connections `/connections`, proxies `/proxies`, graceful kill

### crates/rclash-sys-proxy
- responsibility: system proxy toggle (no TUN in F0)
- key files: `crates/rclash-sys-proxy/src/lib.rs`, `platform::{windows,linux,macos}.rs`
- notable: Windows `winreg`, Linux `gsettings`, macOS `networksetup`

## Feature Layer

### Dashboard
- traffic stats, version, uptime — `src/ui/dashboard.rs`

### Profiles
- import/edit/validate mihomo config.yaml, switch profile triggers core restart — `src/ui/profiles.rs`

### Proxies
- list/select proxies, latency test — `src/ui/proxies.rs`

### Connections
- active connections table, kill — `src/ui/connections.rs`

### Logs
- core stdout/stderr + API logs — `src/ui/logs.rs`

### Settings
- controller address, autostart, theme — `src/ui/settings.rs`

## Packaging
- `packaging/inno/RClash.iss` → setup-RClash-<ver>.exe
- `packaging/deb/` + `cargo deb`, `packaging/rpm/` + `cargo-generate-rpm`, `packaging/appimage/` + `appimagetool`, `packaging/dmg/` + `create-dmg`
- triggered in `.github/workflows/release.yml` on tag `v*`

## Core fork (RClash/mihomo)
- fork of `MetaCubeX/mihomo` branch Alpha, remote `upstream`
- `build-core.yml` matrix 5 triples → `rclash-core-<triple>` + `manifest.json {version, coreSha256: {triple: sha}, buildTime}`
- `sync.yml` cron `0 2 * * *` → `git fetch upstream/Alpha && git merge --no-edit`; success → nightly release; conflict → PR `sync/upstream-YYYY-MM-DD`

## Known Gaps
- TUN mode — deferred to F1 (requires privileged helper)
- Auto-update — manifest.json ready, helper not yet
- Code signing / notarization — secrets optional

## Roadmap
1. F0 scaffolding + CI (current)
2. F1 MVP: profile switch + system proxy + dashboard live data + installers
3. F2: TUN, rule editor, connection kill, latency test, tray, autostart
