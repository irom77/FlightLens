# Release notes

One file per major release, named after its tag (`v0.2.0.md`). A major release
is a new `X.Y.0` or the completion of a [roadmap](../../ROADMAP.md) phase; patch
releases are covered by their [CHANGES.md](../../CHANGES.md) section and the
notes GitHub generates from the commits.

Each file covers what changed for users, anything to know before upgrading
(compatibility with existing `.flightlens` sessions, bundled firmware data,
supported Betaflight versions), and the limitations that remain. After the
Windows and macOS workflows finish, publish the file as the release body:

```sh
gh release edit v<version> --notes-file docs/releases/v<version>.md
```
