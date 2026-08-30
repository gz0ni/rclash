# Commands

## Development

```bash
# install deps
cargo fetch
cargo install cargo-deb cargo-generate-rpm  # linux packaging (optional)

# run locally (debug)
cargo run

# run release
cargo run --release

# sidecar (requires RClash/mihomo built)
go run -C ../mihomo . -d /tmp/rclash-test -f config.yaml
```

## Verification

Run before claiming work is done:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
# go vet only in RClash/mihomo repo
go vet ./...
go test ./...
```

## Build / Package

```bash
# debug build
cargo build

# release build
cargo build --release

# windows installer (requires Inno Setup iscc on PATH or windows-latest)
iscc packaging/inno/RClash.iss

# linux packages
cargo deb
cargo generate-rpm
# appimage: appimagetool packaging/appimage/RClash.AppDir

# macos dmg (requires create-dmg)
create-dmg --volname RClash dist/RClash.dmg dist/RClash.app

# core (in RClash/mihomo repo)
CGO_ENABLED=0 go build -trimpath -ldflags "-s -w -X github.com/metacubex/mihomo/constant.Version=v0.1.0-rclash -X github.com/metacubex/mihomo/constant.MihomoName=RClash -X github.com/metacubex/mihomo/constant.BuildTime=$(date -u +%Y-%m-%dT%H:%M:%SZ)" -o rclash-core
```

## CI

- `.github/workflows/ci.yml` — on push/PR: fmt, clippy, test, build matrix (5 triples)
- `.github/workflows/release.yml` — on tag `v*`: build + package EXE/DEB/RPM/AppImage/DMG + publish Release
- `RClash/mihomo/.github/workflows/build-core.yml` — matrix 5 triples → rclash-core-* + manifest.json
- `RClash/mihomo/.github/workflows/sync.yml` — cron `0 2 * * *` sync upstream/Alpha → nightly or PR

## Tests

- Rust: `cargo test` — unit tests in `crates/*/src` + `src/ui/*`
- Go: `go test ./...` in fork repo
- Where: `tests/` not yet, colocated `#[cfg(test)]` modules
