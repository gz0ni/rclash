# Project Decisions

Recorded decisions and the reasoning behind them, so future work does not silently contradict them. Add an entry whenever a non-trivial choice is made. Latest first.

## Format

```
### <Decision title> (YYYY-MM-DD)

- Decision: <what was chosen, one sentence>
- Reason: <why this and not the alternatives>
- Alternatives considered: <what was rejected and why>
- Consequences: <what this decision affects>
```

---

### GUI stack: eframe 0.32 + egui (2026-08-30)

- Decision: use Rust eframe 0.32 / egui for desktop GUI.
- Reason: cross-platform immediate-mode, small binary, matches project.md toolchain.
- Alternatives considered: Tauri (JS bundle), Flutter (Dart interop overhead).
- Consequences: all UI in `src/ui/*.rs`, need egui theming in Russian.

### Core fork as separate repo RClash/mihomo (2026-08-30)

- Decision: fork MetaCubeX/mihomo into separate repo RClash/mihomo, binary rclash-core, no replace/local Clash.Meta.
- Reason: clean upstream merges, independent releases/nightly, manifest.json with coreSha256 for Helper.
- Alternatives considered: replace in go.mod, vendor Clash.Meta folder like FlClash (pollutes history, merge conflicts).
- Consequences: need sync.yml cron, build-core.yml matrix, manifest contract.

### Core patch ldflags only (2026-08-30)

- Decision: patch only via -X constant.Version/MihomoName/BuildTime, no source edits.
- Reason: keeps fork merge-clean, process name from binary name.
- Alternatives considered: patch constant/*.go (requires manual merge).
- Consequences: Makefile/goreleaser only change, version set at build time.

### CI sync cron 0 2 * * * (2026-08-30)

- Decision: sync.yml cron at 02:00 UTC fetches upstream/Alpha and merges --no-edit; auto nightly on success, PR on conflict.
- Reason: automates staying current without manual sync.
- Alternatives considered: manual sync, weekly cron (staler).
- Consequences: requires upstream remote, nightly releases, PR handling.

### Packaging: InnoSetup EXE + DEB/RPM/AppImage/DMG via CI (2026-08-30)

- Decision: Windows EXE via Inno Setup, Linux DEB/RPM/AppImage, macOS DMG all built in release.yml.
- Reason: covers requested targets, CI-mandatory build.
- Alternatives considered: only cargo build (no installers).
- Consequences: packaging/* templates + CI jobs with iscc/cargo-deb/appimagetool/create-dmg.

### Navigation: 6 tabs minimal (2026-08-30)

- Decision: Dashboard, Profiles, Proxies, Connections, Logs, Settings minimal navigation self-made.
- Reason: user requested no external design, covers core features.
- Alternatives considered: waiting for design mockups.
- Consequences: `src/app.rs` Tab enum, `src/ui/*.rs` stubs.

---

## Example

### State management: Riverpod over Bloc (2026-01-15)

- Decision: use flutter_riverpod for all state.
- Reason: less boilerplate, compile-safe providers, fits the small team.
- Alternatives considered: Bloc (too much ceremony for this size), setState (does not scale to shared settings).
- Consequences: all new state code must be Riverpod; see `.agents/architecture.md`.
