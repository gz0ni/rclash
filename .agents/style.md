# Style

Writing rules for everything the agent produces: answers to the user, commit messages, code comments, documentation. Follow these exactly — no exceptions. If a rule is missing, propose adding it here instead of improvising.

## Language

- Communication with the user: Russian.
- Code identifiers, file names, commit messages, and repo docs: English.
- UI text in the app: Russian (unless the project says otherwise).

## Punctuation (Russian text)

- Long dash is `—` (em dash) with spaces around it: `это — важно`.
- Hyphen only inside words: `кроссплатформенное`.
- Quotes: Russian guillemets `« »`; nested quotes use `„ “`.
- Ellipsis: `…`, not three dots.
- Digits with units: `5 ч`, `30 МБ` (space between number and unit).

## Tone

- Short answers: no filler, no fluff, no polite preambles.
- Answer directly; if asked a question, answer it first, explain after.
- Do not summarize what was done unless asked.

## Code

- No comments unless explicitly requested.
- Keep changes minimal; match existing patterns.
- Never log or print secrets, keys, or tokens.

## Commits

- Short imperative subject line (`Fix usage refresh`, not `Fixed` or `fixing...`).
- Lowercase, no trailing period, English only.

---

<!-- Add project-specific rules below. -->