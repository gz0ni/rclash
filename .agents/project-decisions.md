# Project Decisions

Recorded decisions and the reasoning behind them, so future work does not silently contradict them. Add an entry whenever a non-trivial choice is made. Latest first.

## Format

```
### <Decision title> (YYYY-MM-DD)

- Decision: <what was chosen, one sentence>
- Reason: <why this and not the alternatives>
- Alternatives considered: <what was rejected and why>
- Consequences: <what this decision affects>
```

---

### Subscription User-Agent clash-verge (2026-08-31)

- Decision: все запросы подписок `fetch_and_save_subscription` отправлять с `User-Agent: clash-verge/v2.10.2` (совместим с `clash`).
- Reason: агрегаторы типа `sub.skill-up.store/zL6zB00e5wVrDT5v` отдают полный YAML (`mixed-port`, `dns`, `proxy-groups 🌍 VPN/⚡️ Fastest`, `rules`) только если UA содержит `clash`/`clash-verge`; иначе отдают base64 список ссылок — теряются dns/rules/группы, проверено: без UA 2340 base64, с `clash-verge/v2.10.2` YAML 7890+.
- Alternatives considered: без UA (base64, теряем конфиг), только `Clash` (работает, но `clash-verge` покрывает шире как в FlClash).
- Consequences: `app/src/app.rs:746 SUBS_USER_AGENT` + `app/src/ui/profiles.rs:346` header, будущий `rclash-db` сохраняет `content` целиком.

### TUN 3 OS parallel self-signed gated (2026-08-30)

- Decision: `rclash-tun` (trait `TunStatus/TunBackend`) + `rclash-tun-helper` (`up/down/status/service`) 3 OS параллельно — Linux `pkexec` + `com.rclash.helper.policy` + `ip/iptables`, Win `wintun.dll` + `Service RClashTun`, macOS `utun` + `osascript with administrator privileges` ad-hoc (SMJobBless gated `feature=macos-signed`).
- Reason: пользователь выбрал все 3 параллельно, самоподписи — `SMJobBless` требует $99, заменён на `osascript` для MVP.
- Alternatives considered: последовательно Linux→Win→Mac (медленнее), `SMJobBless` must (блокируется без $99).
- Consequences: `rclash/Cargo.toml` workspace `rclash-tun/helper`, `rclash/crates/rclash-tun/*`, `rclash/crates/rclash-tun-helper`, `rclash-config AppConfig.tun_enabled`, `rclash/src/ui/settings.rs`, `packaging/linux|windows|macos`.

### Code signing gated self-signed (2026-08-30)

- Decision: `release.yml` Windows ephemeral `New-SelfSignedCertificate` → `signtool` (if `WIN_PFX_B64` → trusted, else self-signed), macOS `codesign --sign -` ad-hoc (if `APPLE_CERT_B64` → Developer ID + notary), Android `keytool` debug.jks fallback + `signingConfigs` `apksigner verify`, `docs/SIGNING.md`.
- Reason: пользователь выбрал самоподписи — без $99/EV, CI должен быть зелёным без секретов, но готов к trusted когда появятся.
- Alternatives considered: всегда unsigned (SmartScreen/Gatekeeper worse), всегда требовать секреты (CI падает в форках).
- Consequences: `rclash/.github/workflows/release.yml`, `rclash-android/.github/workflows/release.yml`, `rclash-android/app/build.gradle.kts` signingConfigs, `rclash/docs/SIGNING.md`.

### Auto-update nightly only (2026-08-30)

- Decision: `sync-core.yml` cron `0 4 * * *` зеркалит только `core-nightly` `RClash/mihomo` → `RClash/rclash` (manifest.json + `rclash-core-*`), без `core-stable` канала, `rclash-updater` `sha2` + `atomic` + `chrono`, `App.rs` модалка `UpdateInterval::effective`.
- Reason: пользователь подтвердил nightly достаточно, stable канал избыточен для MVP.
- Alternatives considered: nightly + stable два канала (усложняет manifest).
- Consequences: `.github/workflows/sync-core.yml`, `rclash/crates/rclash-updater`, `rclash/src/app.rs` modal, `rclash/src/ui/{dashboard,settings}.rs`.

### App-wide logger + Logs tabs Core/App (2026-08-30)

- Decision: модуль `rclash/src/logger.rs` `log 0.4` impl `log::Log` с файлом `RClash/logs/app.log` 5МБ ротация + `Mutex<VecDeque>` 2000, `AppConfig.log_level` + `LogLevel::to_log_filter`, вкладка Logs `LogsTab::Core/App` (WS ядро vs app file), Settings ComboBox + кнопка открыть папку, `log::info/warn` во всех App действиях.
- Reason: пользователь попросил логи всего приложения для дебага — WS ядра недостаточно, нужен файл для диагностики у пользователя.
- Alternatives considered: `tracing`+`fern` (тяжелее, дублирует `log` от eframe), `env_logger` только stdout (теряется после закрытия).
- Consequences: `rclash/Cargo.toml` log, `rclash/src/logger.rs`, `rclash/src/main.rs` init, `rclash/crates/rclash-config` LogLevel, `rclash/src/ui/{logs,settings}.rs`, `rclash/src/app.rs` set_log_level.

### Connections WS interval=1000 + Dashboard manual painter (2026-08-30)

- Decision: Connections сразу `WS ws://127.0.0.1:9090/connections?interval=1000` с fallback reconnect 3с (а не REST polling), Dashboard график — ручной `egui::Painter` 140px (линии ↑ синий/↓ зелёный, сетка, 60 точек `VecDeque<f64>`, `format_bytes`) вместо `egui_plot`.
- Reason: пользователь выбрал сразу WS как для traffic/logs; `egui_plot 0.32.1` зависит от `egui 0.31.1` и конфликтует с `eframe 0.32.3` (`ecolor` 0.31 vs 0.32, `Line::new` сигнатура), ручной painter обходит версионный hell и держит бинарь меньше.
- Alternatives considered: `egui_plot 0.32` (несобирается), `egui_plot 0.37` + upgrade eframe 0.36 (ломает 0.32 toolchain), canvas `egui_extras`.
- Consequences: `rclash/Cargo.toml` `tungstenite 0.24`, `rclash/src/app.rs` `ws_traffic_once/ws_connections_once/ws_logs_once` + `channel` + `VecDeque` ring, `rclash/src/ui/{connections,dashboard,logs}.rs`, `rclash/crates/rclash-core-manager` `Snapshot/TrafficInfo/LogEntry/ConnectionsSort/format_bytes`.

### UpdateInterval default 24h with subscription override (2026-08-30)

- Decision: AppConfig.update_interval default H24, если подписка отдаёт интервал (header update-interval или поле yaml update-interval) — берём из подписки, иначе 24ч. Profile mapping UpdateInterval::effective(subscription, app).
- Reason: пользователь уточнил «интервал может отдавать подписка, иначе 24ч» — покрывает оба сценария без ручной настройки.
- Alternatives considered: всегда 24ч (игнор подписки), всегда из подписки (ломает ручную настройку).
- Consequences: `rclash-config/src/lib.rs` UpdateInterval + `AppConfig::update_interval`, `profile::Profile::update_interval`, `custom` atomic, `rclash-subscription` header parsing, будущий `rclash-updater` использует effective.

### Android .aar waits for TUN desktop (2026-08-30)

- Decision: Android fd wiring via CoreBridge + .aar via gomobile стартует только после завершения TUN helper desktop.
- Reason: TUN дизайн (pkexec / wintun / SMJobBless) влияет на fd-прокид и выбор exec vs .aar; параллельная работа рискует переделкой CoreBridge.
- Alternatives considered: параллельно с TUN (быстрее, но риск двойной работы).
- Consequences: `.agents/todo.md` порядок TUN → Android, `mihomo/Makefile` gomobile после TUN, `build-core.yml` aar job после TUN.

### Subscription + Unified Raw as crate + custom.yaml (2026-08-30)

- Decision: крейт `rclash-subscription` (url+percent_decode+base64 url_safe, hysteria2/trojan/vless/vmess/ss, de01.skill-up.store 3 строки, детект yaml/base64/text) + `rclash-config::custom` unified `profiles/custom.yaml` dedup по name/server:port+type, proxy-groups PROXY select, atomic write + CoreApi reload, UI Profiles CollapsingHeader + Proxies крестик, состояние в `RClashApp` via `poll-promise`+`rfd`.
- Reason: покрывает шаг 2 и две отдельные задачи Подписки/Unified Raw одним консистентным потоком; dedup предотвращает дубли; poll-promise не требует tokio runtime в eframe.
- Alternatives considered: отдельный файл на каждую ссылку (раздувает profiles), хранить в памяти (теряется после рестарта), tokio runtime в App (тяжелее).
- Consequences: `rclash/Cargo.toml` rfd/poll-promise, `crates/rclash-subscription`, `crates/rclash-config::{custom,profile}`, `src/app.rs` RClashApp поля, `src/ui/{profiles,proxies}.rs`.

### GUI stack: eframe 0.32 + egui (2026-08-30)

- Decision: use Rust eframe 0.32 / egui for desktop GUI.
- Reason: cross-platform immediate-mode, small binary, matches project.md toolchain.
- Alternatives considered: Tauri (JS bundle), Flutter (Dart interop overhead).
- Consequences: all UI in `src/ui/*.rs`, need egui theming in Russian.

### Core fork as separate repo RClash/mihomo (2026-08-30)

- Decision: fork MetaCubeX/mihomo into separate repo RClash/mihomo, binary rclash-core, no replace/local Clash.Meta.
- Reason: clean upstream merges, independent releases/nightly, manifest.json with coreSha256 for Helper.
- Alternatives considered: replace in go.mod, vendor Clash.Meta folder like FlClash (pollutes history, merge conflicts).
- Consequences: need sync.yml cron, build-core.yml matrix, manifest contract.

### Core patch ldflags only (2026-08-30)

- Decision: patch only via -X constant.Version/MihomoName/BuildTime, no source edits.
- Reason: keeps fork merge-clean, process name from binary name.
- Alternatives considered: patch constant/*.go (requires manual merge).
- Consequences: Makefile/goreleaser only change, version set at build time.

### CI sync cron 0 2 * * * (2026-08-30)

- Decision: sync.yml cron at 02:00 UTC fetches upstream/Alpha and merges --no-edit; auto nightly on success, PR on conflict.
- Reason: automates staying current without manual sync.
- Alternatives considered: manual sync, weekly cron (staler).
- Consequences: requires upstream remote, nightly releases, PR handling.

### Packaging: InnoSetup EXE + DEB/RPM/AppImage/DMG via CI (2026-08-30)

- Decision: Windows EXE via Inno Setup, Linux DEB/RPM/AppImage, macOS DMG all built in release.yml.
- Reason: covers requested targets, CI-mandatory build.
- Alternatives considered: only cargo build (no installers).
- Consequences: packaging/* templates + CI jobs with iscc/cargo-deb/appimagetool/create-dmg.

### Navigation: 6 tabs minimal (2026-08-30)

- Decision: Dashboard, Profiles, Proxies, Connections, Logs, Settings minimal navigation self-made.
- Reason: user requested no external design, covers core features.
- Alternatives considered: waiting for design mockups.
- Consequences: `src/app.rs` Tab enum, `src/ui/*.rs` stubs.

---

## Example

### State management: Riverpod over Bloc (2026-01-15)

- Decision: use flutter_riverpod for all state.
- Reason: less boilerplate, compile-safe providers, fits the small team.
- Alternatives considered: Bloc (too much ceremony for this size), setState (does not scale to shared settings).
- Consequences: all new state code must be Riverpod; see `.agents/architecture.md`.
