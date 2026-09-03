# Gotchas

Traps and pitfalls the agent has stepped on in this project. Log a gotcha immediately whenever something does not build, run, or pass — right after the incident, while context is fresh, do not wait for session end. Check this file before repeating similar work. One entry per trap.

## Format

```
### <Short trap title> (YYYY-MM-DD)

- What happened / what was wrong
- Root cause
- How to avoid next time
```

---

<!-- New entries go here, newest first. -->

### egui Area/Window reuses persisted rect by id (2026-09-03)

- Input dialog rendered wider than its fixed content suggested.
- Root cause: suspect — `Area`/`Window` with id `input_dialog` reused a persisted rect from the earlier `Window` with the same id; egui/eframe remembers rects per id.
- Avoid: give rebuilt overlays a fresh id (e.g. `input_dialog_v2`); cap content with `set_max_width`.

### cargo test fails while debug exe is running (2026-09-03)

- `cargo test --workspace` failed to link: `failed to remove file target\debug\rclash.exe`, os error 5 (access denied).
- Root cause: running `rclash.exe` (PID 26348) locks the binary; cargo cannot replace it.
- Avoid: close the running app before `cargo test --workspace`; lib-only tests (`cargo test -p <crate>`) work while it runs.

## Example

### Riverpod provider was not overridden in tests (2026-01-15)

- Tests crashed with ProviderNotFoundException.
- Root cause: `settingsRepositoryProvider` must be overridden in `main()` and in tests via `ProviderScope`.
- Avoid: always override providers in widget tests; see `.agents/commands.md` for the test setup.