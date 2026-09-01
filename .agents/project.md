# Project Context

RClash — desktop proxy suite, 100% mihomo compat. Inno Setup style topory desktop only.

- Desktop: Rust egui (eframe 0.32), core — fork MetaCubeX/mihomo as Go sidecar (rclash-core).
- Android removed — desktop only (2026-09-01).

## Filesystem layout (monorepo root `RClash/`)

- `src/`, `crates/`, `packaging/` — desktop (monorepo root, git `gz0ni/rclash` → `RClash/rclash`)
- `core/` — subtree `MetaCubeX/mihomo` `Alpha` `--squash` (no .git), ldflags-only

## Platforms
- Desktop: Windows / Linux / macOS (x64 + arm64) fixed window 860×620 Inno style
- iOS: deferred

## Toolchain
- Rust 1.77+ / eframe 0.32, egui 0.32
- Go 1.22+ (current 1.26.5), CGO_ENABLED=0
- Packaging: Inno Setup 6 (Windows EXE), cargo-deb / cargo-generate-rpm / appimagetool (Linux), create-dmg (macOS)

## Repositories
- `RClash/rclash` — monorepo root (here, `src/` + `core/` subtree)

## Core patch policy
- No source patch: only `go build -C core -ldflags "-X constant.Version=v0.1.0-rclash -X constant.MihomoName=RClash -X constant.BuildTime=..." -o target/.../rclash-core-<triple>`
- Binary name gives process name
- Updates via `git subtree pull --prefix=core https://github.com/MetaCubeX/mihomo Alpha --squash` + PR `sync/upstream-YYYY-MM-DD` (manual `tag v*` after merge), cron `sync-subtree.yml` 0 2 * * *

## Status
F0 scaffolding done — desktop Inno style 860x620 fixed + 3 themes Light/Dark/OLED + CI desktop only
