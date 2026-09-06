# Agent rules

- Do not use Superpowers skills (`superpowers:*`) in this repository. Work directly with the available tools and other relevant skills.
- Keep FlightLens focused on inspecting, comparing, and auditing FPV flight controller backups offline. Preserve offline operation.
- Read the relevant code and repository configuration before making changes. Follow existing conventions.
- Keep changes scoped to the requested task; avoid unrelated refactors and new dependencies unless needed.
- Preserve existing user changes. Do not overwrite unrelated work or run destructive Git commands without explicit authorization.
- Treat flight controller backups as read-only inputs unless the user explicitly requests modification. Keep backup contents and secrets out of logs and external services.
- Run the checks relevant to the change using the repository's documented commands or configuration. Report any checks that could not run.
- Summarize what changed, how it was verified, and any remaining limitations. Commit or push only when requested.
- Work in checkpoints rather than one long uninterrupted run. After each self-contained unit of work (a completed feature slice, a passing check, a group of related edits), stop and hand control back so the user can compact the conversation, review the diff, and commit or push before the next unit begins.
- At each checkpoint, state briefly what changed, what was verified, and what the next unit would be; then wait for the user instead of continuing automatically.
