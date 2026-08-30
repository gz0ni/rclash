# Bugs

Known bugs found in this project and how they were fixed, so they do not reappear. Logged at the end of every session. Status: `open` / `fixed` / `won't fix`.

## Format

```
### <Short bug title> (YYYY-MM-DD) — status

- Symptom
- Root cause
- Fix applied
- How to prevent recurrence
```

---

<!-- New entries go here, newest first. -->

## Example

### Usage screen shows stale limits after refresh (2026-01-15) — fixed

- Symptom: limits did not update after pressing refresh.
- Root cause: provider cached state and never invalidated.
- Fix: added `ref.invalidate(usageProvider)` on refresh action.
- Prevent: always invalidate providers after async mutations.