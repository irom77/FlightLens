# Mode-name evidence

D3 is implemented using the permanent IDs and default display names in
Betaflight's `src/main/msp/msp_box.c` table. These are the permanent IDs used by
CLI AUX assignments, not the internal sequential `boxId_e` enum values.

`crates/flightlens-core/compatibility/mode-names.json` bundles exact tables for
4.2.0–4.2.11, 4.3.0–4.3.2, 4.4.0–4.4.3, 4.5.0–4.5.5,
2025.12.1–2025.12.5, 4.5.3.KAACK_V19 and
2025.12.3-alpha.KAACK_V19. Every entry records an immutable source URL and SHA-256.
For example, the [4.5.0 source](https://github.com/betaflight/betaflight/blob/c155f5830d0ffdee1c34071dd21f174ffc374c81/src/main/msp/msp_box.c)
defines 45 mode names, including LAP TIMER RESET, absent in 4.2.

Run `python3 tools/verify_mode_names.py --check` to download only public firmware
source and reproduce the bundle without modifying it. Without `--check`, the
tool regenerates the bundle for review. It excludes commented-out historical
entries and rejects duplicate IDs. No app operation uses the network.

Runtime selection requires an exact version match, including vendor suffixes.
An ID absent from a verified table stays `Mode ID N`. Other versions retain the
previous 19 fallback labels without extending those assumptions. This labels
stored assignments; it does not assert that the board compiled or enabled the
feature. Custom USER1–4 display names are outside this change.

Parser regression coverage exercises release-specific labels, removed IDs,
newer-only IDs, vendor identity, unverified suffixes and unknown numeric IDs.
Stored IDs, ranges, source text and export commands are unchanged.
