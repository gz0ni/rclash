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

- [ ] Org: создать GitHub org `RClash` (делегировано агенту) — проверить доступность, создать, пригласить gz0ni как Owner
- [ ] Repo: создать/проверить `RClash/rclash` (десктоп) + `RClash/mihomo` (форк MetaCubeX/mihomo Alpha), настроить remotes upstream/origin, ветка rclash
- [ ] Docs: заполнить `.agents/project.md|architecture.md|commands.md|rules.md|models.md`
- [ ] Scaffolding: Cargo workspace + eframe 0.32 App с 6 вкладками (Dashboard, Profiles, Proxies, Connections, Logs, Settings)
- [ ] Core-manager: spawn `rclash-core`, healthcheck REST 9090, stop/restart, логи в UI
- [ ] Config: serde_yaml модели mihomo, валидация, хранение dirs::config_dir/RClash
- [ ] Sys-proxy: абстракция winreg / networksetup / gsettings
- [ ] Ядро CI: `RClash/mihomo/.github/workflows/build-core.yml` (матрица 5 триплетов, CGO_ENABLED=0, ldflags -X constant.Version/MihomoName/BuildTime, бинарь rclash-core-<triple>, manifest.json с coreSha256)
- [ ] Ядро CI: `sync.yml` cron 0 2 * * * (fetch upstream/Alpha → merge --no-edit → nightly release / PR sync/upstream-YYYY-MM-DD)
- [ ] Десктоп CI: `RClash/rclash/.github/workflows/ci.yml` (check/clippy/test/build матрицы)
- [ ] Десктоп CI: `release.yml` (tag v* → EXE InnoSetup + DEB/RPM/AppImage/DMG)
- [ ] Упаковка: `packaging/inno/RClash.iss`, deb/rpm/appimage/dmg скрипты, iscc/create-dmg/appimagetool
- [ ] Верификация: `cargo clippy -D warnings && cargo test && go vet`, проверка sha256 manifest ↔ артефактов, dry-run sync.yml

## In Progress

- [~] Org: создание GitHub org RClash — проверка + создание (gz0ni)
- [~] Repo: инициализация локального git + создание GitHub репо

## Done

- [x] Решения зафиксированы: навигация 6 вкладок, инсталляторы, ldflags-патч, CI-синк (2026-08-30)
- [x] План F0 согласован (2026-08-30)
