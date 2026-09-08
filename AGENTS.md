# Agent rules

- Do not use Superpowers skills (`superpowers:*`) in this repository. Work directly with the available tools and other relevant skills.
- Keep FlightLens focused on inspecting, comparing, and auditing FPV flight controller backups offline. Preserve offline operation.
- Read the relevant code and repository configuration before making changes. Follow existing conventions.
- Keep changes scoped to the requested task; avoid unrelated refactors and new dependencies unless needed.
- Preserve existing user changes. Do not overwrite unrelated work or run destructive Git commands without explicit authorization.
- Treat flight controller backups as read-only inputs unless the user explicitly requests modification. Keep backup contents and secrets out of logs and external services.
- Run the checks relevant to the change using the repository's documented commands or configuration. Report any checks that could not run.
- Summarize what changed, how it was verified, and any remaining limitations. Commit or push only when requested.
- Keep `TODO.md` and `CHANGES.md` current as part of the work, not as a cleanup pass afterwards. Move an item out of `TODO.md` when it ships, and add what the work uncovered but did not fix.
- Record every user-visible change in `CHANGES.md` under `## Unreleased`, in the same commit that makes it. Leave out changes no user or maintainer can observe, such as formatting, comments, or test-only refactors.
- Treat every push as a release, because the desktop installers are built from what is pushed. Before pushing: run `./test.sh` and `pnpm screenshots:check`, bump the version in `package.json`, `src-tauri/tauri.conf.json` and `Cargo.toml` (refresh `Cargo.lock` with `cargo check`), rename the `## Unreleased` heading in `CHANGES.md` to the new version and date, then push the commit and a matching `v<version>` tag. The tag is what starts `.github/workflows/windows-release.yml` and `.github/workflows/macos-release.yml`, which build and publish the Windows and macOS installers for that version.
- Ask before pushing without a version bump and tag. A commit that reaches `main` untagged ships to nobody, so say so instead of leaving it unreleased silently.
- A major release, meaning a new `X.Y.0` or the completion of a `ROADMAP.md` phase, also needs release notes in `docs/releases/v<version>.md`: what changed for users, upgrade and compatibility notes, and known limitations. Publish them as the release body once the workflows finish, with `gh release edit v<version> --notes-file docs/releases/v<version>.md`. Patch releases rely on the `CHANGES.md` section and the generated notes.
- Work in checkpoints rather than one long uninterrupted run. After each self-contained unit of work (a completed feature slice, a passing check, a group of related edits), stop and hand control back so the user can compact the conversation, review the diff, and commit or push before the next unit begins.
- At each checkpoint, state briefly what changed, what was verified, and what the next unit would be; then wait for the user instead of continuing automatically.
