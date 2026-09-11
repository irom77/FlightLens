# Betaflight 2025.12 compatibility investigation

Investigated 2026-09-11. This is an implementation checkpoint, not certification of the KAACK fork. Only public upstream source was fetched; no backup contents were uploaded.

## Available source pins

`git ls-remote --tags https://github.com/betaflight/betaflight.git 'refs/tags/*2025*'` returned RC1–RC4 and plain releases 2025.12.1 through 2025.12.5, but no 2025.12.0 tag. Use 2025.12.1 as the first plain release when extending the generator, rather than assuming a `.0` tag exists. Verified pins: 2025.12.1 = `85d201376a1fc33b223c27448808c2cc7b8f2743`; 2025.12.3 = `db7df6e48b9727d5984e18c906bf0e4769b2abf1`; 2025.12.5 = `7348054f268f0058574719c134e9f149565bb8ea`. [Official tags](https://github.com/betaflight/betaflight/tags)

## Serial syntax is a separate compatibility change

The 2025.12.3 CLI prints named ports in `serial` commands. It first resolves names case-insensitively, then accepts numeric identifiers; numeric values below 20 are translated from the legacy UART numbering. Thus simply normalizing names to numbers risks confusing the old and new numbering. [CLI printSerial and cliSerial](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/cli/cli.c#L1258-L1325)

Upstream identifiers include VCP = 20, SOFT1/2 = 30/31, LPUART1 = 40, UART0 = 50 where available, UART1 = 51, and PIOUART0 = 70. Names are build-dependent entries in `serialPortNames`. The function mask adds GIMBAL at bit 18 relative to 4.5.0. Preserve semantic port identity and firmware-appropriate export syntax; test named ports and legacy numeric input. [Serial identifiers and masks](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/io/serial.h), [Serial names and lookup](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/io/serial.c#L147-L223)

## Generator and analysis findings

The existing schema regex matches 679 setting declarations and 67 lookup-table entries at each of 2025.12.1, .3, and .5, versus 598 declarations and 63 tables at 4.5.0. The declaration format remains compatible with the generator's initial extraction, but this count does not prove every bound, enum or conditional setting is resolved. Profile counts in `common_pre.h` are four PID and four rate profiles at the three inspected 2025.12 tags. [Settings](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/cli/settings.c), [Lookup enum](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/cli/settings.h), [Profile counts](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/target/common_pre.h)

The four rate function bodies extracted by `tools/build_rate_vectors.py`—Betaflight, Kiss, Actual and Quick—are byte-identical between 4.5.0 and 2025.12.1. Extend and compile differential vectors before claiming support; this inspection does not verify the custom fork or every patch. [4.5.0 rate functions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/fc/rc.c), [2025.12.1 rate functions](https://github.com/betaflight/betaflight/blob/85d201376a1fc33b223c27448808c2cc7b8f2743/src/main/fc/rc.c)

The rate reset function appears identical across inspected .1, .3 and .5 releases and includes `thrHover8`. The generator must still prove invariance across **all** plain patches before certifying omitted defaults. A suffixed custom/prerelease version must continue to receive no certified defaults: upstream source cannot establish what KAACK changed. [Rate reset](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/fc/controlrate_profile.c)

OSD coordinate packing retains the 4.5 layout: five low X bits, five Y bits, extra HD X bit at 10, visibility bits starting at 11 and type bits at 14–15. Other OSD enums changed, including timer sources and warnings, so coordinate compatibility does not certify all OSD interpretation. [2025.12.3 OSD definitions](https://github.com/betaflight/betaflight/blob/db7df6e48b9727d5984e18c906bf0e4769b2abf1/src/main/osd/osd.h), [4.5.0 definitions](https://github.com/betaflight/betaflight/blob/4.5.0/src/main/osd/osd.h)

## Serial checkpoint completed

Named ports, case-insensitive lookup, legacy numeric aliases, and the Gimbal mask
are implemented for 2025.12 inspection. Serial export token round trips are tested;
the compatibility-pack gate still disables actual 2025.12 export. Full checks pass
against the current 14-file corpus, with all 15 original serial errors removed.

## Remaining checkpoint

Generate a pinned 2025.12 schema, audit unresolved constants and lookup changes, verify all release resets and compiled rate vectors, update pack selection and supported-version UI copy, and test custom builds without default inference. Keep generated compatibility data bundled for offline use.
