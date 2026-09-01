# RClash — desktop (monorepo)

Monorepo — `RClash` is single git `gz0ni/rclash` (target `RClash/rclash`), `core/` is `subtree` of `MetaCubeX/mihomo` `Alpha` `--squash`.

```
RClash/  (monorepo root, Rust 1.77 + Go 1.22)
├─ src/            # desktop — Rust eframe 0.32, 860×620 Inno topory
├─ crates/         # rclash-config, core-manager, subscription, sys-proxy, autostart, tun, db, updater
├─ core/           # subtree MetaCubeX/mihomo Alpha (no .git), ldflags-only
├─ packaging/      # inno, appimage, dmg
└─ .github/        # ci.yml + release.yml (5 triples) + sync-subtree.yml
```

## Quick start

```bash
# desktop (root)
cargo run
cargo build --release

# core (subtree)
go build -C core -o target/debug/rclash-core -ldflags "-X github.com/metacubex/mihomo/constant.Version=v0.1.0-rclash -X github.com/metacubex/mihomo/constant.MihomoName=RClash -X github.com/metacubex/mihomo/constant.BuildTime=$(date -u +%Y-%m-%dT%H:%M:%SZ)" .
go vet ./core/...
```

## CI

- `.github/workflows/ci.yml` → fmt, clippy, test, build (5 triples)
- `.github/workflows/release.yml` `on: tag v*` → `go build -C core` (5) + `cargo build` + `iscc`/`cargo deb`/`create-dmg` → `dist/setup-*.exe` + `rclash-core-*` + `manifest.json` → Release
- `.github/workflows/sync-subtree.yml` `cron 0 2 * * *` → `git subtree pull --prefix=core https://github.com/MetaCubeX/mihomo Alpha --squash` → PR `sync/upstream-YYYY-MM-DD` (manual `tag v*` after merge)
