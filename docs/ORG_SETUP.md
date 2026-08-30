# Org setup — RClash

Current fallback: `gz0ni/rclash` + `gz0ni/mihomo` (branch `rclash`).

Desired final: `RClash/rclash` + `RClash/mihomo`.

## Why fallback

GitHub org `RClash` cannot be created via API (`admin:org` + browser device flow required). Token `gist, read:org, repo` insufficient and `POST /orgs` returns 404 for free tier. Creation is UI-only: https://github.com/account/organizations/new

## To migrate after creating org

1. Create org `RClash` via browser (Free, no enterprise needed).

2. Transfer repos:

```bash
gh repo transfer RClash/rclash --recipient RClash   # for gz0ni/rclash
gh repo transfer RClash/mihomo --recipient RClash   # for gz0ni/mihomo
# or via web: Settings → Danger Zone → Transfer ownership
```

3. Update remotes:

```bash
cd /path/to/rclash
git remote set-url origin https://github.com/RClash/rclash.git
cd /tmp/mihomo-fork
git remote set-url origin https://github.com/RClash/mihomo.git
```

4. Update `manifest.json` consumers if any hardcode `gz0ni/mihomo` → `RClash/mihomo`.

No code change needed — binary `rclash-core` and ldflags `MihomoName=RClash` are org-agnostic.

## Fork maintenance

```bash
git remote add upstream https://github.com/MetaCubeX/mihomo.git
git fetch upstream
git checkout rclash
git merge --no-edit upstream/Alpha   # sync.yml does this daily at 02:00 UTC
```
