# Project Context

RClash — desktop + Android proxy suite, 100% mihomo compat.

- Desktop: Rust egui (eframe 0.32), core — fork MetaCubeX/mihomo as Go sidecar (rclash-core).
- Android: Kotlin VpnService + Compose, same core, APK via Release.

## Filesystem layout (D:\Work\Project\RClash container)

- `rclash/` — desktop app (this repo, git `gz0ni/rclash` → `RClash/rclash`)
- `mihomo/` — fork MetaCubeX/mihomo (git `gz0ni/mihomo` → `RClash/mihomo`), branch `rclash`
- `rclash-android/` — Android app (git `gz0ni/rclash-android` → `RClash/rclash-android`)

## Platforms
- Desktop: Windows / Linux / macOS (x64 + arm64)
- Android: arm64 + amd64 (minSdk 24), APK only, no AAB/Play
- iOS: deferred

## Toolchain
- Rust 1.77+ / eframe 0.32, egui 0.32
- Go 1.22+ (current 1.26.5), CGO_ENABLED=0
- Android: JDK 17, Gradle 8.7, Android SDK 34, Kotlin 1.9.22, Compose BOM 2024.06.00
- Packaging: Inno Setup 6 (Windows EXE), cargo-deb / cargo-generate-rpm / appimagetool (Linux), create-dmg (macOS), APK (Android)

## Repositories
- `RClash/rclash` — desktop (here, `rclash/`)
- `RClash/mihomo` — fork core (`mihomo/`), binary `rclash-core`, ldflags-only patch
- `RClash/rclash-android` — Android (`rclash-android/`)

## Core patch policy
- No source patch: only `go build -ldflags "-X constant.Version=v0.1.0-rclash -X constant.MihomoName=RClash -X constant.BuildTime=..." -o rclash-core-<triple>`
- Binary name gives process name
- Updates via CI sync `sync.yml` cron 0 2 * * *, not manual

## Status
F0 scaffolding done — desktop 6 tabs + Android VpnService stub + CI for all three repos + packaging stubs
