# Project Context

RClash — desktop proxy suite, 100% mihomo compat. Inno Setup style topory desktop only.

- Desktop: Rust egui (eframe 0.32), core — fork MetaCubeX/mihomo as Go sidecar (rclash-core).
- Android removed — desktop only (2026-09-01).

## Filesystem layout (D:\Work\Project\RClash container)

- `app/` — desktop app (git `gz0ni/rclash` → `RClash/rclash`)
- `core/` — fork MetaCubeX/mihomo (git `gz0ni/mihomo` → `RClash/mihomo`), branch `rclash`

## Platforms
- Desktop: Windows / Linux / macOS (x64 + arm64) fixed window 860x620 Inno style
- iOS: deferred

## Toolchain
- Rust 1.77+ / eframe 0.32, egui 0.32
- Go 1.22+ (current 1.26.5), CGO_ENABLED=0
- Packaging: Inno Setup 6 (Windows EXE), cargo-deb / cargo-generate-rpm / appimagetool (Linux), create-dmg (macOS)

## Repositories
- `RClash/rclash` — desktop (here, `app/`)
- `RClash/mihomo` — fork core (`core/`), binary `rclash-core`, ldflags-only patch

## Core patch policy
- No source patch: only `go build -ldflags "-X constant.Version=v0.1.0-rclash -X constant.MihomoName=RClash -X constant.BuildTime=..." -o rclash-core-<triple>`
- Binary name gives process name
- Updates via CI sync `sync.yml` cron 0 2 * * *, not manual

## Status
F0 scaffolding done — desktop Inno style 860x620 fixed + 3 themes Light/Dark/OLED + CI desktop only
