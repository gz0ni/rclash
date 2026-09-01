# TODO

Task checklist. The agent maintains this file: creates items, marks them done, keeps it honest. Work from here — do not lose tasks between sessions.

## Format

- `[ ]` pending task — add due date or note if needed
- `[x]` done task — keep for history, do not delete immediately
- `[~]` in progress — who/what is working on it

## Rules

- Add a task the moment it is discovered, even if vague.
- Mark `[x]` only when actually verified done (checks passed), not by intent.
- Review this file at the end of every session: close finished items, add follow-ups.

---

## Open

- [ ] iOS — в долгом ящике

## In Progress

## Done

- [x] Inno топорный desktop 860×620 + 3 темы OLED + тонкие рамки 1px + монохром иконки hover (2026-09-01) — фикс окно `main.rs:860×620 non-resizable` + `AppConfig theme Light/Dark/Oled default Dark` + `show_traffic_graph` + `theme_visuals/border_color_for` `Rounding::ZERO` `Stroke 1px #000/#FFF` + `RClashApp overlay RawKeys/Editor + loc_fav/Search` + `CentralPanel ScrollArea` Group `Профили (Combo + ↻ + + → 4-меню + ✎ + ☰ + ⚙)` `Прокси (Combo groups + ◎ Пинг)` `Локации Grid 4 страны/тип/пинг/radio+★` `График 80px if show` `Трафик прием/отдача/ip` `Режим rule/global/direct + Система proxy/tun + автообновление` + `Window Введите текст ниже (multiline) + Window Добавить (буфер/url/файл/сырой)` + `Config/RawKeys/Editor/Logs/Settings (4 таба Приложение/Ядро/DNS/Сеть BOLT + ? хелпы hover)` монохром иконки `← ↻ ✎ ⚙ ☰ ◎ ★ ☆ ? ×` `on_hover_text` + `cargo fmt/check/clippy/test` ok 36 passed + `android/` удалён + `ci/release` без android
- [x] Удаление Android — `android/` `ci.yml android-lint` `release.yml build-android` + доки `project.md/architecture.md/README` — контейнер `app/core` только desktop (2026-09-01)

- [x] Single Column + BottomNav FlClash — slate #334155 + amber только CTA #D97706, cyan #0891B2 indigo #6366F1, Single Column scroll (full width) + BottomNav 4 56px (2026-08-31) — `app/src/app.rs:916 Top 42` + `CentralPanel ScrollArea::vertical full width` CTA 320×56 `Подключить #0891B2 / Отключить #D97706` + `ПРОФИЛЬ #192134` + `ЛОКАЦИИ` (поиск/фильтры/группы `extract_groups` + `selected_group` + ★ `rclash-db`) без `max_height 280` + `ТРАФИК` 100px painter `↑ slate #334155 / ↓ cyan #0891B2` `max 1024` `format_bytes` + `TopBottomPanel bottom 56` 4 `◉ Локации / ⧉ Конфиг / ★ Избранное / ⚙ Настройки` — фикс дубли BottomNav/оверлея (1426 + 1928 дубль → один, 1455+1951 дубль → один), `delay_color 755` cyan/gray без amber/red, `mockup/index.html 59 w-full` + `145 bg-cta→bg-primary` только big CTA, удалён `app/src/ui/` + `main.rs mod ui`, `cargo fmt/check/clippy/test` ok 36 passed (full width по запросу)
- [x] B Bento 2×2 — максимально отличить от B.O.L.T (2026-08-31) — `mockup/index.html` 4 карты bento grid `amber #D97706 + indigo #6366F1 + cyan #0891B2` Background `#0F172A` Card `#192134` `variance 8 density 8`, без кольца ⏻ → прямоугольная CTA `Подключить #0891B2 / Отключить #D97706` 320×56, `app/src/app.rs:1245` ring → Button, `delay_color` fast `#0891B2` медленный `#D97706`, `proxy-groups` табы `Все группы | 🌍 VPN | ⚡️ Fastest | ★ Избранное` `extract_groups` + `selected_group` + фильтр по `all`, `★` fav `rclash-db` persist, `cargo check/clippy` ok
- [x] БД для конфигов как FlClash (2026-08-31) — `app/crates/rclash-db` `rusqlite 0.31 bundled` `rclash.db` WAL `configs(id,name,url,content,hash,is_active,created_at,updated_at)` + `favorites(proxy_name PK,group_name,added_at)` хранить весь YAML `content` целиком (пример `mixed-port:7890 dns fake-ip 198.18.0.1/16 9 proxies hysteria2/trojan/vless reality 2 proxy-groups 🌍/⚡️ rules`), `save_config/update_content/set_active/delete_config`, `migrate_from_files` `profiles.json/custom.yaml`, `Cargo.toml` members + `open="5.3"` + `rclash-config` dep, `cargo test -p rclash-db` ok
- [x] Редактор конфига внутри + открыть в системе (2026-08-31) — `app/src/app.rs` `editor_name/content/error` + `Overlay Config` `Редактор YAML (БД)` `TextEdit multiline 12 rows mono` `💾 Сохранить` `rclash_db::update_content` + `reload_core` + `↗ Открыть в системе` `open::that(tmp_rclash_*.yaml)` + `watch`, `fetch_and_save_subscription` также `save_config` в БД
- [x] P3 UI порт одноэкранный B.O.L.T (2026-08-31) — `app/src/app.rs` Tab 6 → `Overlay {None,Config,Settings,Logs}` + `LocFilter {All,Fav,Fast}` + `loc_search/fav/selected_proxy`, top 42px `RClash v0.1.0-beta ● Online + ⧉≡⚙`, left 320px `ЛОКАЦИИ` поиск + фильтры + список singles с delay badge + ★/○ + radio `PUT /proxies/PROXY`, center кольцо 200px power toggle `sys_proxy 127.0.0.1:7890` + `Подключено Germany02-Hysteria2` + профиль `admin` карточка + график 60 точек WS /traffic, bottom 64px 4 метрики, overlays Window 520px Config/Settings/Logs, helpers `extract_singles/format_bytes/delay_text/reload_core/fetch_and_save_subscription`, `ui/*.rs` dead_code allowed, `logger.rs` allow, `1.0_f32` Clippy MSRV 1.77, `cargo check/clippy/test` ok
- [x] P3 Мокап `mockup/index.html` Tailwind B.O.L.T (2026-08-31) — статика `html+css+js` Tailwind CDN, header `RClash v0.1.0-beta • Online` + 3 кнопки `⧉≡⚙`, left 320px ЛОКАЦИИ поиск + фильтры Все/★/● + список 6 локаций с 77мс/★/○, center кольцо 200px `Подключено` + admin карточка + bottom 4 метрики Приём/Отдача/Время/DE, overlays Config `profiles/custom.yaml` + Settings + Logs drawer, `app.js` мок-данные, интерактив выбор/тест задержек
- [x] P2 Монорепо `RClash/` = корень (2026-08-31) — `rclash→app`, `mihomo→core`, `rclash-android→android`, `.github/workflows/{ci.yml,release.yml}` в корень, `release.yml` unified: `build-desktop` 5 триплетов Go 1.22 + Rust + `rclash-core` бандл + `iscc/packaging` + portable zip, `build-android` JDK17 Gradle APK + `publish` merge, `ci.yml` `cargo --manifest-path app/Cargo.toml` + `go vet core` + `android lint`
- [x] Подписка User-Agent clash-verge (2026-08-31) — проверено `sub.skill-up.store/zL6zB00e5wVrDT5v`: без UA base64 2340 (`aHlzd...`), с `clash-verge/v2.10.2` YAML `mixed-port: 7890` + `proxy-groups` + `rules`, фикс `app/src/app.rs:746 SUBS_USER_AGENT` + `app/src/ui/profiles.rs:346` header `clash-verge/v2.10.2`, `project-decisions.md:18` решение
- [x] P4 Верификация (2026-08-31) — `cargo fmt --check` ok, `cargo check --all-targets` ok, `cargo clippy -D warnings` ok, `cargo test --workspace` 35 passed, `go vet ./...` core ok, `mockup/index.html` 34K ok
- [x] P1 Банндл ядра — бандл `rclash-core-<triple>` рядом с `rclash` (2026-08-31) — `core_binary_name/resolve_core_path() exe_dir→config_dir→PATH` `app/crates/rclash-core-manager/src/process.rs:5, lib.rs:4 Cargo.toml dirs`, убрать `rclash-updater` из `app/Cargo.toml:32`, `app.rs` updater поля+методы+окна `app/src/ui/settings.rs:257` → бандл инфо + `core_path`, `.github/workflows/release.yml` `setup-go 1.22` + 5 триплетов `go build -C core -ldflags Version/MihomoName/BuildTime -o app/target/.../rclash-core`, `app/packaging/appimage/build.sh:6+ dmg/build.sh:7 + inno RClash.iss:33` core рядом, `cargo check/clippy/test` ok
- [x] TUN helper (F1 desktop) — 3 OS параллельно: crates rclash-tun (lib TunStatus/TunBackend) + rclash-tun-helper bin up/down/status/service, Linux pkexec com.rclash.helper.policy + ip/iptables, Win wintun.dll + Service RClashTun, macOS utun + osascript admin (ad-hoc, SMJobBless gated), AppConfig tun_enabled + Settings checkbox (Linux pkexec / Win Service / macOS osascript) + logger, packaging/linux|windows|macos, cargo check/clippy/test ok + helper compiles (2026-08-30)
- [x] Auto-update Helper — manifest.json coreSha256 RClash/rclash core-nightly: crate rclash-updater (fetch_manifest/check_for_update/download_and_verify sha2 atomic + chrono last_check, triple linux-amd64 etc, core_bin_path), sync-core.yml cron 0 4 * * * mirror RClash/mihomo nightly → core-nightly, App.rs updater modal «Доступно vX — установить?» [Обновить/Позже/Пропустить версию] (AppConfig skipped_version/last_check/update_interval effective), Dashboard/Settings check button, cargo check/clippy/test ok (2026-08-30)
- [x] Code signing / notarization gated self-signed (MVP): release.yml Windows ephemeral New-SelfSignedCertificate → signtool + dist/*.exe (if WIN_PFX_B64 → trusted), macOS ad-hoc codesign --sign - (if APPLE_CERT_B64 → Developer ID + notary), Linux no sign, Android keystore self-signed (keytool genkey debug.jks fallback, signingConfigs release, apksigner verify) — docs/SIGNING.md, cargo check/clippy/test ok (2026-08-30)
- [x] Android fd wiring prep (ждёт TUN desktop API) — CoreBridge.kt useAar try System.loadLibrary("rclash") + nativeStartWithFd fallback exec, RClashVpnService fd прокид, app/build.gradle.kts signingConfigs release + release.jks gated, docs/ANDROID_AAR_FD.md + mihomo gomobile bind docs, cargo check/clippy/test ok (2026-08-30)
- [x] UI intuitive: шаг 3 — Connections таблица WS ?interval=1000 (сортировка Время/↑/↓/Хост, фильтр, закрыть ❌, close_all DELETE /connections) + Logs WS /logs?level= + уровни debug/info/warning/error/silent, фильтр/поиск/автоскролл/очистить/копировать + App логи (file RClash/logs/app.log 5МБ + in-memory 2000, Settings LogLevel, Logs вкладка App/Core, открыть папку) — tungstenite 0.24, format_bytes/log_level_color, cargo check/clippy/test ok (2026-08-30)
- [x] UI intuitive: шаг 4 — Dashboard график трафика ring 60 точек WS /traffic (вместо egui_plot — ручной painter 140px, линии ↑ синий/↓ зелёный, пик KB/с, total, max, автоскролл 1с) — manual egui painter из-за egui 0.32 vs egui_plot 0.31 mismatch, cargo check/clippy/test ok (2026-08-30)
- [x] UI intuitive: шаг 2 — Profiles список/empty + Proxies карточки/режим (Правило/Глобальный/Прямой) + задержка цвет + интеграция подписок/Raw unified (сырые ссылки в один custom.yaml с удалением ключей) + AppConfig update_interval/skipped_version/last_check + CoreApi ProxyMode/delay + rfd/poll-promise — cargo check/clippy/test ok (2026-08-30)
- [x] Подписки — крейт rclash-subscription (url+percent_decode+base64 url_safe, hysteria2/trojan/vless/vmess/ss, de01.skill-up.store 3 строки, детект yaml/base64/text → отдельные профили) — cargo test ok (2026-08-30)
- [x] Unified Raw — profiles/custom.yaml единый конфиг, dedup по name/server:port+type, proxy-groups PROXY select, Profiles CollapsingHeader + [🗑 Удалить] + Proxies карточки с крестиком, atomic write + CoreApi reload — cargo test ok (2026-08-30)
- [x] UI intuitive: шаг 1 — widgets (card/badge/empty) + Dashboard карточки (3 колонки: Ядро/Трафик/Действия) + Settings секции (Общие/Ядро/Сеть/О программе) — cargo check/clippy/test ok (2026-08-30)
- [x] UI polish: темы персистенс (rclash-config AppConfig), трей (tray-icon) hide-on-close + --minimized, автозапуск (rclash-autostart: Win Run, Linux autostart/desktop, macOS LaunchAgents) + CI tray deps (2026-08-30)
- [x] F1 polish: CoreApi traffic/proxies/connections/reload + secret, Dashboard alive/version, темы светлая/тёмная, Settings sys-proxy toggle (2026-08-30)
- [x] Верификация после реорганизации: cargo check/clippy/test ok (rclash/), check 4/5 build ✓, android lint ✓ (2026-08-30)
- [x] Fix Android CI: gradle.properties android.useAndroidX=true (24a889f) (2026-08-30)
- [x] Move .agents/AGENTS.md/opencode.jsonc to container RClash/ (c2c1bc0) (2026-08-30)
- [x] Реорганизация файловой системы: RClash/ как контейнер с rclash/ + mihomo/ + rclash-android/ (2026-08-30)
- [x] Android: отдельный репо gz0ni/rclash-android (main e56a855) — Kotlin VpnService + Compose 6 tabs, gradle 8.7, CI APK (2026-08-30)
- [x] Org fallback: gz0ni/rclash + gz0ni/mihomo + gz0ni/rclash-android → RClash/* после создания org, docs/ORG_SETUP.md (2026-08-30)
- [x] Repo: gz0ni/rclash (main da0df04), fork gz0ni/mihomo rClash db2b63a3, Makefile NAME=rclash-core + ldflags MihomoName=RClash (2026-08-30)
- [x] Docs: project.md|architecture.md обновлены под контейнер и Android (2026-08-30)
- [x] Scaffolding desktop: Cargo workspace + eframe 0.32 App 6 вкладок (2026-08-30)
- [x] Crates: rclash-config, rclash-core-manager, rclash-sys-proxy (2026-08-30)
- [x] Ядро CI: build-core.yml 5 триплетов + manifest.json, sync.yml cron 0 2 * * * (2026-08-30)
- [x] Десктоп CI: ci.yml + release.yml 5 triples + packaging stubs (2026-08-30)
- [x] Упаковка desktop: inno/RClash.iss, appimage/dmg (2026-08-30)
- [x] Верификация локальная: cargo fmt/check/clippy/test ok, go build rclash-core v0.1.0-rclash ok (2026-08-30)
