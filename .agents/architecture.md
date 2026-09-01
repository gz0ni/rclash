# Architecture

## Filesystem

```
RClash/  (monorepo root)
├─ src/        # desktop (monorepo)
├─ crates/     # rclash-*
├─ core/       # subtree MetaCubeX/mihomo Alpha --squash
└─ packaging/  # inno/appimage/dmg
```

## Entry Point (desktop)
`src/main.rs` → `RClashApp` (eframe::App). Window fixed 860x620 non-resizable, theme Light/Dark/Oled (default Dark), thin 1px borders. Initializes `CoreManager`, `AppConfig`, `SysProxy`, `TrayIcon` and runs `eframe::run_native`. `--minimized` flag hides window on autostart.

## Core Layer (desktop)

### app
- responsibility: top-level egui app, Inno 860×620, 1px mono groups
- key files: `src/app.rs`, `src/main.rs`
- notable: `RClashApp {overlay, loc_* , settings_tab}`, `theme_visuals/border_color_for`, `Frame 1px`

### crates/rclash-config
- responsibility: mihomo YAML parsing, validation, profile persistence + `AppConfig {theme Light/Dark/Oled default Dark, show_traffic_graph bool, minimize_to_tray, skipped_version, last_check, update_interval}` persist `app.json` + `profiles.json` + `profiles/custom.yaml` unified RAW + PatchClashConfig mirror BOLT
- key files: `crates/rclash-config/src/lib.rs`, `crates/rclash-config/src/profile.rs`, `crates/rclash-config/src/custom.rs`
- notable: `serde_yaml`/`serde_json` models, stored in `dirs::config_dir()/RClash` (Win `%APPDATA%/RClash`, Linux `~/.config/rclash`, macOS `~/Library/Application Support/RClash`), `Theme Light/Dark/Oled` thin 1px borders (Light #000000 Dark/Oled #FFFFFF), `UpdateInterval {Manual,30m,1h,6h,12h,24h}` default 24h with subscription override via `UpdateInterval::effective`, atomic write `tmp+rename`, `Profile {name,path,url,update_interval,last_update,is_raw}`, `ProfileStore {profiles,active}`, `custom.yaml` dedup `name/server:port+type` + `proxy-groups PROXY select`

### crates/rclash-core-manager
- responsibility: spawn/manage `rclash-core` sidecar, REST client to mihomo API
- key files: `crates/rclash-core-manager/src/lib.rs`, `crates/rclash-core-manager/src/process.rs`, `crates/rclash-core-manager/src/api.rs`
- notable: `Command::new("rclash-core")`, healthcheck `GET http://127.0.0.1:9090/version`, traffic WS `/traffic`, connections `/connections`, proxies `/proxies` + `GET /proxies/{name}/delay` + `PATCH /configs {mode}` + `DELETE /connections`, `ProxyMode {Rule,Global,Direct}`, `CoreApi {base,secret,client}` with `Authorization: Bearer`

### crates/rclash-subscription
- responsibility: subscription import — детект yaml/base64/text → отдельные профили, парсинг сырых ссылок hysteria2/trojan/vless/vmess/ss
- key files: `crates/rclash-subscription/src/lib.rs`
- notable: `url`+`percent-encoding`+`base64` url_safe + pad, `detect_format` yaml vs base64 vs text, `parse_raw_link` per scheme, `parse_text_links` + `parse_subscription_content`

### crates/rclash-sys-proxy
- responsibility: system proxy toggle
- key files: `crates/rclash-sys-proxy/src/lib.rs`, `platform::{windows,linux,macos}.rs`
- notable: Windows `winreg`, Linux `gsettings`, macOS `networksetup`

### crates/rclash-autostart
- responsibility: autostart enable/disable per OS
- key files: `crates/rclash-autostart/src/{lib,windows,linux,macos}.rs`
- notable: Win `HKCU\...\Run` `"RClash"="\"exe\" --minimized"`, Linux `~/.config/autostart/RClash.desktop`, macOS `~/Library/LaunchAgents/com.rclash.app.plist`

### tray
- responsibility: system tray icon + hide-on-close + --minimized
- key files: `src/tray.rs`, `src/app.rs` (tray poll + close_requested → Visible(false)), `src/main.rs` (init_tray)
- notable: `tray-icon 0.12`, icon 32x32 RGBA generated, menu Show/Exit, DoubleClick → show, requires `libgtk-3-dev libappindicator3-dev libxdo-dev` on Linux CI

## Feature Layer (desktop)

### app — `src/app.rs`
- responsibility: Inno style fixed window, group frames 1px, monochrome icons with hover, help ? tooltips, BOLT settings
- key files: `src/app.rs`
- notable: `RClashApp {overlay: None|Editor|Logs|Settings|RawKeys, app_config{theme Light/Dark/Oled show_traffic_graph}, core_version/alive, profile_store, raw_keys, import_url/interval/raw_text, proxies_data/mode, proxy_delays}` via `poll-promise::Promise::spawn_thread` + `reqwest::blocking`, `rfd`, helpers `group_frame/icon_btn_mono/help_btn`

## Packaging
- Desktop: `packaging/inno/RClash.iss` → `setup-RClash-<ver>.exe`, `cargo deb`/`cargo-generate-rpm`/`appimagetool`/`create-dmg` via `.github/workflows/release.yml` on tag `v*` (monorepo 5 triples + `go build -C core`)

## Core (subtree)

- subtree `MetaCubeX/mihomo` `Alpha` `--squash` at `core/` (no .git), `go build -C core -ldflags Version/MihomoName/BuildTime -o target/.../rclash-core`
- sync via `.github/workflows/sync-subtree.yml` `cron 0 2 * * *` → `git subtree pull --prefix=core ... Alpha --squash` → PR `sync/upstream-YYYY-MM-DD` (manual `tag v*` after merge)

## Known Gaps
- TUN desktop — done MVP (Linux pkexec, Win Service+wintun, macOS utun+osascript, AppConfig tun_enabled, helper binary) — самоподпись gated, SMJobBless deferred
- Auto-update — done MVP (crate rclash-updater sha2 atomic, sync-core.yml nightly mirror, App.rs modal Dashboard/Settings, interval effective) — stable channel deferred
- Code signing / notarization — done MVP gated self-signed (Windows ephemeral PFX, macOS ad-hoc, docs/SIGNING.md) — trusted certs deferred until secrets
- iOS — deferred (long box)

## Roadmap
1. F0 scaffolding + CI (done: desktop + core)
2. F1 MVP: profile switch + system proxy + dashboard live data + installers + tray/autostart/theme (done 2026-08-30: tray+autostart+theme)
3. F2: Inno fixed 860x620 + 3 themes + BOLT settings + TUN/rule editor, connection kill, latency test
