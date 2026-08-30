# Model Catalog and Routing

Active as of 2026-08-30.

## Routing

| Task class | Model to propose |
|---|---|
| Default coding | opencode-go/kimi-k2.7-code |
| Complex refactoring, planning | opencode-go/glm-5.3 |
| Long autonomous work | opencode-go/kimi-k3 |
| Vision / UI debugging | opencode-go/mimo-v2.5-pro |
| Code review | opencode-go/glm-5.3 (subagent code-reviewer) |
| Exploration / research | explore (flash) |
| Routine (tests, simple edits) | deepseek-v4-flash |

## Workflow

1. Classify the task against the table.
2. If the current session model does not fit, propose a switch in one line: `Switch to /models -> <model> for this task.`
3. Delegate exploration and review to subagents instead of the main model.
4. Never burn top-tier models on routine work.
