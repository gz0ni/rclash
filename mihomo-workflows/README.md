# RClash/mihomo workflows

These two files belong in `RClash/mihomo` (fork of MetaCubeX/mihomo), not in this repo.

- `build-core.yml` → `.github/workflows/build-core.yml`
- `sync.yml` → `.github/workflows/sync.yml`

Setup after fork:

```bash
gh repo fork MetaCubeX/mihomo --org RClash --fork-name mihomo --clone=false
gh repo clone RClash/mihomo /tmp/mihomo
cd /tmp/mihomo
git remote add upstream https://github.com/MetaCubeX/mihomo.git
git fetch upstream
git checkout -b rclash upstream/Alpha
# minimal patch: rename binary in Makefile/goreleaser if any
cp /path/to/RClash/mihomo-workflows/*.yml .github/workflows/
git add .github/workflows/ && git commit -m "ci: add build-core and sync workflows"
git push -u origin rclash
```

Build: `CGO_ENABLED=0 go build -trimpath -ldflags "-s -w -X github.com/metacubex/mihomo/constant.Version=v0.1.0-rclash -X github.com/metacubex/mihomo/constant.MihomoName=RClash -X github.com/metacubex/mihomo/constant.BuildTime=$(date -u +%Y-%m-%dT%H:%M:%SZ)" -o rclash-core .`
