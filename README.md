# RClash

Desktop proxy client, 100% mihomo compatible. GUI Rust egui (eframe 0.32), core fork `RClash/mihomo` (Go sidecar `rclash-core`).

## Quick start

```bash
cargo run
cargo build --release
```

## Packaging

- Windows: `iscc packaging/inno/RClash.iss` → `dist/setup-RClash-*.exe` (Inno Setup 6)
- Linux: `cargo deb`, `cargo generate-rpm`, `bash packaging/appimage/build.sh`
- macOS: `bash packaging/dmg/build.sh aarch64-apple-darwin`

## Core

Fork `MetaCubeX/mihomo` as `RClash/mihomo`. Patch only via ldflags:

```
-X github.com/metacubex/mihomo/constant.Version=v0.1.0-rclash
-X github.com/metacubex/mihomo/constant.MihomoName=RClash
-X github.com/metacubex/mihomo/constant.BuildTime=...
```

Binary name `rclash-core` gives process name. Sync via `sync.yml` cron `0 2 * * *`.

Workflows for the fork are in `mihomo-workflows/` — copy to `RClash/mihomo/.github/workflows/`.

## CI

- `ci.yml` — fmt, clippy, test, build 5 triples on push/PR
- `release.yml` — on tag `v*` builds + packages EXE/DEB/RPM/AppImage/DMG
