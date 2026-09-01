# Org setup — RClash (monorepo)

Current: `gz0ni/rclash` (monorepo root `src/` + `core/` subtree `MetaCubeX/mihomo`).

Desired final: `RClash/rclash` (single repo, core via `git subtree`).

`gz0ni/mihomo` fork no longer needed — `core/` is `subtree` directly from `MetaCubeX/mihomo`.

## Why fallback

GitHub org `RClash` cannot be created via API (`admin:org` + browser device flow required). Token `gist, read:org, repo` insufficient and `POST /orgs` returns 404 for free tier. Creation is UI-only: https://github.com/account/organizations/new

## To migrate after creating org

1. Create org `RClash` via browser (Free, no enterprise needed).

2. Transfer repos:

```bash
gh repo transfer gz0ni/rclash --new-owner RClash   # or web: Settings → Danger Zone → Transfer
# no separate mihomo fork needed
```

3. Update remotes:

```bash
cd /path/to/RClash
git remote set-url origin https://github.com/RClash/rclash.git
```

No code change — binary `rclash-core` `MihomoName=RClash` org-agnostic.

## Core subtree maintenance (monorepo)

```bash
git subtree pull --prefix=core https://github.com/MetaCubeX/mihomo Alpha --squash -m "sync core $(date +%F)"
# or via CI: .github/workflows/sync-subtree.yml cron 0 2 * * * → PR sync/upstream-YYYY-MM-DD
```
