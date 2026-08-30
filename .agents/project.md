# Project Context

RClash — desktop proxy client, 100% mihomo compat. GUI Rust egui (eframe 0.32), core — fork MetaCubeX/mihomo as Go sidecar (rclash-core).

## Platforms
Windows / Linux / macOS (x64 + arm64)

## Toolchain
- Rust 1.77+ / eframe 0.32, egui 0.32
- Go 1.22+ (current 1.26.5), CGO_ENABLED=0
- Packaging: Inno Setup 6 (Windows EXE), cargo-deb / cargo-generate-rpm / appimagetool (Linux), create-dmg (macOS)

## Repositories
- `RClash/rclash` — desktop app (this repo)
- `RClash/mihomo` — fork of MetaCubeX/mihomo (separate repo), binary `rclash-core`, ldflags-only patch

## Core patch policy
- No source patch: only `go build -ldflags "-X constant.Version=v0.1.0-rclash -X constant.MihomoName=RClash -X constant.BuildTime=..." -o rclash-core-<triple>`
- Binary name gives process name
- Updates via CI sync, not manual

## Status
F0 scaffolding — workspace + 6 tabs navigation + CI skeleton + packaging stubs
