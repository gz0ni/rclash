# Architecture

## Filesystem

```
D:\Work\Project\RClash\          # container (not git)
├─ rclash/                       # desktop (git gz0ni/rclash)
├─ mihomo/                       # core fork (git gz0ni/mihomo, branch rclash)
└─ rclash-android/               # android (git gz0ni/rclash-android)
```

## Entry Point (desktop)
`rclash/src/main.rs` → `rclash::app::RClashApp` (eframe::App). Initializes `CoreManager`, `AppConfig`, `SysProxy` and runs `eframe::run_native`.

## Core Layer (desktop)

### app
- responsibility: top-level egui app, tab routing, global state
- key files: `rclash/src/app.rs`, `rclash/src/main.rs`
- notable: `enum Tab { Dashboard, Profiles, Proxies, Connections, Logs, Settings }`, SidePanel navigation, CentralPanel content

### crates/rclash-config
- responsibility: mihomo YAML parsing, validation, profile persistence
- key files: `rclash/crates/rclash-config/src/lib.rs`, `rclash/crates/rclash-config/src/profile.rs`
- notable: `serde_yaml` models, stored in `dirs::config_dir()/RClash` (Win `%APPDATA%/RClash`, Linux `~/.config/rclash`, macOS `~/Library/Application Support/RClash`)

### crates/rclash-core-manager
- responsibility: spawn/manage `rclash-core` sidecar, REST client to mihomo API
- key files: `rclash/crates/rclash-core-manager/src/lib.rs`, `rclash/crates/rclash-core-manager/src/process.rs`, `rclash/crates/rclash-core-manager/src/api.rs`
- notable: `Command::new("rclash-core")`, healthcheck `GET http://127.0.0.1:9090/version`, traffic WS `/traffic`, connections `/connections`, proxies `/proxies`, graceful kill

### crates/rclash-sys-proxy
- responsibility: system proxy toggle (no TUN in F0)
- key files: `rclash/crates/rclash-sys-proxy/src/lib.rs`, `platform::{windows,linux,macos}.rs`
- notable: Windows `winreg`, Linux `gsettings`, macOS `networksetup`

## Feature Layer (desktop)

### Dashboard — `rclash/src/ui/dashboard.rs`
### Profiles — `rclash/src/ui/profiles.rs`
### Proxies — `rclash/src/ui/proxies.rs`
### Connections — `rclash/src/ui/connections.rs`
### Logs — `rclash/src/ui/logs.rs`
### Settings — `rclash/src/ui/settings.rs`

## Android (rclash-android/)

- `RClashVpnService.kt` — `VpnService` `establish()` tun fd → `CoreBridge.startWithFd`
- `CoreBridge.kt` — exec `rclash-core` sidecar (future `.aar` via gomobile), `protectSocket`
- `MainActivity.kt` — Compose `TabRow` 6 tabs mirror desktop (Dashboard/Profiles/Proxies/Connections/Logs/Settings)
- `AndroidManifest.xml` — `BIND_VPN_SERVICE`, `FOREGROUND_SERVICE`, `INTERNET`

## Packaging
- Desktop: `rclash/packaging/inno/RClash.iss` → `setup-RClash-<ver>.exe`, `cargo deb`/`cargo-generate-rpm`/`appimagetool`/`create-dmg` via `rclash/.github/workflows/release.yml` on tag `v*`
- Android: `rclash-android/app/build.gradle.kts` → `APK` via `rclash-android/.github/workflows/release.yml` on tag `v*` (no AAB/Play)

## Core fork (mihomo/)

- fork of `MetaCubeX/mihomo` branch Alpha, remote `upstream`, path `mihomo/` (not `RClash/mihomo` until org)
- `mihomo/.github/workflows/build-core.yml` matrix 5 desktop + 2 android triples → `rclash-core-<triple>` + `rclash.aar` + `manifest.json {version, coreSha256: {triple: sha}, buildTime}`
- `mihomo/.github/workflows/sync.yml` cron `0 2 * * *` → `git fetch upstream/Alpha && git merge --no-edit`; success → nightly; conflict → PR `sync/upstream-YYYY-MM-DD`

## Known Gaps
- TUN desktop — deferred to F1
- Auto-update — manifest.json ready, helper not yet
- Code signing / notarization — secrets optional
- iOS — deferred (long box)

## Roadmap
1. F0 scaffolding + CI (done: desktop + android + core)
2. F1 MVP: profile switch + system proxy + dashboard live data + installers + Android VpnService fd wiring
3. F2: TUN, rule editor, connection kill, latency test, tray, autostart, Android .aar
