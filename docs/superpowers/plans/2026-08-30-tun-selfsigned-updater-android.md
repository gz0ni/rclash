# TUN + Auto-update + Signing + Android Plan

> **Model:** opencode/muse-spark-1.2-free-xhigh for all tasks

**Goal:** TUN helper 3 OS параллельно (Linux pkexec, Win wintun+Service, macOS utun+osascript) с self-signed gated, nightly mirror sync-core.yml + rclash-updater, Android .aar fd prep.

**Tech:** Rust 1.77, eframe 0.32, tungstenite 0.24, log, sha2, anyhow, windows-service/wintun (cfg), utun (cfg), reqwest blocking.

**Spec:** todo.md:21-24, architecture.md gaps, project-decisions.

## File Structure

- `rclash/Cargo.toml` workspace + new members `crates/rclash-tun`, `crates/rclash-tun-helper`, `crates/rclash-updater`
- `rclash/crates/rclash-tun/src/{lib,linux,windows,macos,common}.rs` trait TunBackend
- `rclash/crates/rclash-tun-helper/src/main.rs` up/down/status (pkexec on Linux)
- `rclash/crates/rclash-updater/src/lib.rs` check/download/verify
- `rclash/src/app.rs` tun_enabled, updater poll, modal
- `rclash/src/ui/{settings,dashboard}.rs` TUN toggle, update modal
- `rclash/crates/rclash-config/src/lib.rs` AppConfig tun_enabled
- `.github/workflows/release.yml` gated self-signed steps
- `docs/SIGNING.md` self-signed instructions
- `.github/workflows/sync-core.yml` nightly mirror
- `rclash-android/` gomobile stub, CoreBridge fd API docs

## Tasks

### TUN scaffolding
- Create crates, workspace members, TunBackend trait, TunStatus

### TUN Linux
- pkexec policy `packaging/linux/com.rclash.helper.policy`, helper uses ip/iptables, rtnetlink tun0

### TUN Windows
- wintun.dll bundling, windows-service RClashTun, self-signed generation in CI if no secrets

### TUN macOS
- utun device + osascript with administrator privileges fallback, ad-hoc codesign

### App integration
- AppConfig tun_enabled bool, Settings checkbox enabled, yaml tun section, logger

### Auto-update
- rclash-updater crate sha2 verify, atomic, sync-core.yml, UI modal

### Signing gated
- release.yml if secrets != '' else generate ephemeral self-signed, docs/SIGNING.md

### Android prep
- Makefile gomobile bind docs, CoreBridge startWithFd JNI stub

### Verify
- cargo fmt/check/clippy/test, docs updates
