# Rules

## Code Style
- Rust: `cargo fmt` (default), `clippy -D warnings` must pass
- Go (fork): `go fmt`, `go vet`
- No comments unless requested; match existing patterns; keep changes minimal

## Languages
- User-facing UI text: Russian.
- Identifiers, file names, commits, docs: English.

## Design
- Minimal navigation: Dashboard, Profiles, Proxies, Connections, Logs, Settings — do not invent extra tabs without approval
- Follow egui/eframe idioms: immediate mode, no retained widget abstraction
- Do not invent new state crate — use `crates/rclash-config`, `crates/rclash-core-manager`, `crates/rclash-sys-proxy` only

## State and Persistence
- Config via `dirs::config_dir()/RClash` + `serde_yaml`/`serde_json`; no DB in F0
- Core state via mihomo REST API (`http://127.0.0.1:9090`), not via file polling

## Testing
- `cargo test` must stay green; add `#[cfg(test)]` for new logic where reasonable
- Do not mock egui context in tests — test logic crates, not UI rendering

## Generated and External Files
- Do not edit `Cargo.lock` manually; do not commit `target/`, `dist/`
- Do not patch `constant/*.go` in fork — only ldflags; keep mihomo sources pristine for clean merges

## Git
- Do not commit, push, create PRs, or run destructive git operations without explicit user request.
- Before any commit (if requested): check `git status`, `git diff`, `git log --oneline -10`; stage only intended files.
- Fork sync: `git fetch upstream && git merge --no-edit upstream/Alpha`; on conflict create PR `sync/upstream-YYYY-MM-DD`, never force-push Alpha

## Packaging
- Binary name is `rclash-core` (gives process name); desktop binary `rclash` / `RClash`
- manifest.json format is contract: `{version, buildTime, coreSha256: {triple: sha256}}` — do not change keys without updating Helper
