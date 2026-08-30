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

- [ ] TUN helper (F1 desktop) — привилегированный helper для TUN
- [ ] Auto-update Helper на основе manifest.json coreSha256 (desktop + android)
- [ ] Code signing / notarization для EXE/DMG/APK (secrets)
- [ ] UI polish: темы, трей, автозапуск (desktop) + Android fd wiring via CoreBridge
- [ ] Android .aar via gomobile (замена exec sidecar если tun fd не пройдёт)
- [ ] iOS — в долгом ящике

## In Progress

- [~] Верификация после реорганизации в RClash/{rclash,mihomo,rclash-android} — cargo check + CI

## Done

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
