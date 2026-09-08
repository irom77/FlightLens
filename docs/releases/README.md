# Release policy and notes

FlightLens uses `MAJOR.MINOR.PATCH` (`X.Y.Z`), following
[Semantic Versioning](https://semver.org/) with the pre-1.0 policy below.
Choose the bump from the changes since the previous release, using the highest
applicable category. Release frequency and patch count do not determine it.
This policy applies to future releases; existing versions and tags stay as published.

## Choosing a version

| Release | Trigger | FlightLens examples |
| --- | --- | --- |
| Patch (`0.2.0 → 0.2.1`, `1.2.0 → 1.2.1`) | Compatible corrections and maintenance of existing behavior, including small UI polish. | Fix parsing for already supported backups, incorrect rate calculations, audit false positives, or installer failures. |
| Minor (`0.2.x → 0.3.0`, `1.2.x → 1.3.0`) | New user capabilities or compatible additions to supported behavior. After 1.0, also use minor for documented deprecations. | Add workspace comparison, firmware support, audit rules, or export capabilities. |
| Breaking change before 1.0 (`0.2.x → 0.3.0`) | Change an existing compatibility contract while the product is still in initial development. | Change saved-session compatibility or remove supported inputs. Describe the break and upgrade path explicitly. |
| First stable release (`0.y.z → 1.0.0`) | Commit to a dependable, documented compatibility contract. | Core inspection workflows are reliable, supported firmware boundaries are clear, and saved-data compatibility has an explicit policy. |
| Major after 1.0 (`1.y.z → 2.0.0`) | Break an established compatibility contract. | Make existing sessions require manual conversion, change documented export formats incompatibly, or remove previously supported inputs. |

SemVer permits unstable behavior throughout `0.y.z`; FlightLens makes the
stronger promise that patches remain compatible. Minor releases before 1.0 can
contain breaking changes, which must be called out in their release notes.
Reset patch to zero for a minor bump, and minor and patch to zero for a major
bump. A release may include changes from lower categories.

## Compatibility and 1.0 readiness

The user contract covers supported backup inputs, saved sessions and settings,
and documented export formats and behavior. Preserve offline analysis and
read-only source backups across all versions; a major bump does not authorize
changing those product boundaries.

Internal Rust/React refactors or a visual redesign do not alone require a major
bump. A correction to an incorrect calculation or audit can change results and
still be a patch: explain consequential corrections in the release notes.
Adding a capability is a minor change even if its implementation is small.
An automatic migration that preserves users' saved data and workflows need not
be breaking; document any upgrade or downgrade limitations.

Before 1.0, document the supported inputs, saved-data compatibility and migration
policy, and known limitations, and verify the core inspection workflows through
the release checks and relevant desktop smoke checks. Version 1.0 represents
confidence in those commitments, not completion of every roadmap phase.
After 1.0, compatible features continue to use minor bumps regardless of size.

## Cadence

- Release urgent fixes promptly after verification, especially incorrect analysis
  or exports.
- Batch routine patches roughly weekly or fortnightly when useful fixes exist.
- Release a coherent new capability when ready. During active development,
  4–8 weeks is a planning target for minor releases, not a deadline.
- There is no patch limit: `0.2.17` is valid if those releases correct existing
  functionality. A new capability can justify `0.3.0` immediately after `0.2.0`.
- Roadmap phase completion warrants milestone notes but does not itself choose
  the version bump. Reserve “major” for a change to `X`; call a new `X.Y.0` a
  feature release or milestone when speaking collectively.

## Release notes

Create `docs/releases/v<version>.md` for every new `X.Y.0` release and for any
release completing a [roadmap](../../ROADMAP.md) phase. Ordinary patch releases
use their [CHANGES.md](../../CHANGES.md) section and GitHub-generated notes.

Each release-note file covers:

- What changed for users.
- Upgrade and compatibility notes: saved sessions/settings (including
  `.flightlens` sessions when supported), bundled firmware data, supported
  Betaflight versions, and documented exports. Explicitly identify any breaking
  changes and required user action, or state that no breaking changes are known.
- Known limitations that remain.

Follow the [release checklist](../../DEVELOPMENT.md#releases) to verify, version,
tag, and publish the release. After both Windows and macOS workflows finish,
publish the milestone notes as the release body:

```sh
gh release edit v<version> --notes-file docs/releases/v<version>.md
```

Published versions and tags are immutable. Ship corrections under a new version.
