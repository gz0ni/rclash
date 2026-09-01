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

## Example

### Riverpod provider was not overridden in tests (2026-01-15)

- Tests crashed with ProviderNotFoundException.
- Root cause: `settingsRepositoryProvider` must be overridden in `main()` and in tests via `ProviderScope`.
- Avoid: always override providers in widget tests; see `.agents/commands.md` for the test setup.