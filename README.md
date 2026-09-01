# RClash — desktop

Filesystem container (not a git repo). Each subfolder is its own git repo (fallback `gz0ni/*`, target `RClash/*`).

```
RClash/
├─ app/            # desktop — Rust eframe 0.32, https://github.com/gz0ni/rclash
├─ core/           # core fork — Go, https://github.com/gz0ni/mihomo (branch rclash)
└─ (android removed — desktop only)
```

See `app/docs/ORG_SETUP.md` for transfer to `RClash` org.

## Quick start

```bash
# desktop
cargo run --manifest-path app/Cargo.toml

# core (in core/)
go build -C core -o ../app/target/debug/rclash-core -ldflags "-X github.com/metacubex/mihomo/constant.Version=v0.1.0-rclash -X github.com/metacubex/mihomo/constant.MihomoName=RClash -X github.com/metacubex/mihomo/constant.BuildTime=$(date -u +%Y-%m-%dT%H:%M:%SZ)" .
```

## CI

- `.github/workflows/ci.yml` + `release.yml` → EXE/DEB/RPM/AppImage/DMG (5 desktop triples)
- `core/.github/workflows/build-core.yml` + `sync.yml` → rclash-core-* + manifest.json
