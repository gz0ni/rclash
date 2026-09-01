# AGENTS.md

Agent entry point for this repository. Keep this file small; detailed context lives in `.agents/*.md` and repo-scoped skills live in `.agents/skills/*/SKILL.md`.

Communication with the user: Russian. Code, commits, and documentation: English.

## Start Here

Read these files before making changes:

- [.agents/project.md](.agents/project.md): project overview, platforms, versions, dependencies, design basis.
- [.agents/commands.md](.agents/commands.md): dev, test, analyze, build, and packaging commands.
- [.agents/rules.md](.agents/rules.md): coding, testing, and workflow conventions.
- [.agents/style.md](.agents/style.md): writing style for all text you produce (dashes, quotes, tone, formatting).
- [.agents/todo.md](.agents/todo.md): current task checklist — read it, work from it, update it.

Read these only when the task touches their area:

- [.agents/architecture.md](.agents/architecture.md): code structure, key modules, known gaps.
- [.agents/gotchas.md](.agents/gotchas.md): traps you (the agent) stepped on before — check before repeating work.
- [.agents/bugs.md](.agents/bugs.md): known bugs and their fixes, so they do not reappear.
- [.agents/project-decisions.md](.agents/project-decisions.md): why things are done this way.
- [.agents/models.md](.agents/models.md): model catalog and routing for this project.
- [.agents/skills.md](.agents/skills.md): index of repo-scoped skills.

## Highest Priority Rules

- Read `.agents/*.md` before starting work; follow them.
- Verify with the project's checks (see `.agents/commands.md`) before claiming work is complete.
- Keep changes minimal and follow existing patterns.
- Do not add comments unless requested.
- Do not commit, push, or create PRs without explicit user request.
- UI text: Russian; code identifiers, commits, docs: English.
- All text you write (answers, commits, docs) must follow `.agents/style.md`.

## Agent Maintenance Duties

You maintain these files yourself — no user prompting needed:

- **Whenever something does not build/run/pass**: log the gotcha immediately in `.agents/gotchas.md` (what happened, root cause, how to avoid) while context is fresh.
- **End of every session**: append new bugs to `.agents/bugs.md` (with status), record decisions in `.agents/project-decisions.md` (decision → date → reason).
- **Every task**: update `.agents/todo.md` — create items, mark done, keep it honest.
- **When project facts change**: update the static files (`project.md`, `commands.md`, `architecture.md`, `rules.md`).

## Repo Skills

Use repo skills from `.agents/skills/` when a task matches their descriptions. See [.agents/skills.md](.agents/skills.md).