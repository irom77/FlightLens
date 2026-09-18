# Agent rules

## Overview & Guidelines
This document defines operational protocols, execution workflows, and tool usage rules for AI agents operating within this repository.

---

## 1. General Directives
- Do not use Superpowers skills (`superpowers:*`) in this repository. Work directly with the available tools and other relevant skills.
- Keep FlightLens focused on inspecting, comparing, and auditing FPV flight controller backups offline. Preserve offline operation.
- Read the relevant code and repository configuration before making changes. Follow existing conventions.
- Keep changes scoped to the requested task; avoid unrelated refactors and new dependencies unless needed.
- Preserve existing user changes. Do not overwrite unrelated work or run destructive Git commands without explicit authorization.
- Treat flight controller backups as read-only inputs unless the user explicitly requests modification. Keep backup contents and secrets out of logs and external services. Feedback may place a redacted configuration on the user's clipboard for them to submit themselves; that is the user's action, not transmission by the app.
- Run the checks relevant to the change using the repository's documented commands or configuration. Report any checks that could not run.
- Summarize what changed, how it was verified, and any remaining limitations. Commit or push only when requested.
- Keep `TODO.md` and `CHANGES.md` current as part of the work, not as a cleanup pass afterwards. Move an item out of `TODO.md` when it ships, and add what the work uncovered but did not fix.
- Record every user-visible change in `CHANGES.md` under `## Unreleased`, in the same commit that makes it. Leave out changes no user or maintainer can observe, such as formatting, comments, or test-only refactors.
- Treat every push as a release, because the desktop installers are built from what is pushed. Before pushing: run `./test.sh` and `pnpm screenshots:check`, bump the version in `package.json`, `src-tauri/tauri.conf.json` and `Cargo.toml` (refresh `Cargo.lock` with `cargo check`), rename the `## Unreleased` heading in `CHANGES.md` to the new version and date, then push the commit and a matching `v<version>` tag. The tag is what starts `.github/workflows/windows-release.yml` and `.github/workflows/macos-release.yml`, which build and publish the Windows and macOS installers for that version.
- Ask before pushing without a version bump and tag. A commit that reaches `main` untagged ships to nobody, so say so instead of leaving it unreleased silently.
- Before choosing a version, preparing release notes, or publishing a release, read and follow [the release policy](docs/releases/README.md). It defines bump criteria, compatibility, cadence, and required milestone notes. Choose the bump from the changes being released, never from patch count or elapsed time.
- Work in checkpoints rather than one long uninterrupted run. After each self-contained unit of work (a completed feature slice, a passing check, a group of related edits), stop and hand control back so the user can compact the conversation, review the diff, and commit or push before the next unit begins.
- At each checkpoint, state briefly what changed, what was verified, and what the next unit would be; then wait for the user instead of continuing automatically.

---

## 2. Development Execution Modes

We support two distinct modes of execution: **Standard Mode** (default) and **Adversarial Mode** (on-demand).

### Mode A: Standard Execution (Default)
By default, execute instructions directly:
1. Plan and apply necessary file modifications.
2. Run standard tests and linting (`pnpm test` and/or `cargo test` inside `src-tauri/`).
3. Report results directly to the user.

---

### Mode B: Adversarial Execution (On-Demand Only)

Run the multi-round adversarial harness **only** when explicitly requested by the user.

#### Trigger Keywords & Flags
Activate this mode if the prompt contains any of the following:
- CLI Flags: `--adversarial`, `-adv`, `/adversarial`
- Key Phrases:
  - `"use adversarial review"`
  - `"run with reviewer"`
  - `"adversarial mode"`
  - `"two personas / coder and reviewer"`
  - `"critique and refine"`

#### Roles & Personas
- **Coder Persona (`.codex/prompts/coder.md`):** Implements code changes, writes comprehensive tests, and iterates based strictly on reviewer feedback without introducing regressions.
- **Reviewer Persona (`.codex/prompts/reviewer.md`):** Acts as a security- and edge-case-focused auditor. Inspects `git diff` and execution logs. Does not write code; outputs either `STATUS: REJECTED` with specific critiques or `STATUS: APPROVED`.

#### Execution Workflow
When triggered:
1. **Tree Cleanliness:** Check `git status` to verify there are no uncommitted collisions.
2. **Launch Harness:** Execute the automated runner with the stripped task prompt:
   ```bash
   python .codex/scripts/adversarial_review.py "<TASK_SPECIFICATION>"