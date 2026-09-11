use crate::{compatibility, model::*, source};
use std::collections::{BTreeMap, BTreeSet};

/// `MAX_AUX_CHANNEL_COUNT` in `rx/rx.h`: the 18 supported RC channels less the
/// four stick channels. Identical in every Betaflight line FlightLens certifies,
/// and not build-dependent, so it is a constant rather than a schema field.
pub const AUX_CHANNEL_COUNT: u32 = 14;

/// Read back the values the source left out.
///
/// A Betaflight diff prints only what differs from the reset it names, so on a
/// document whose `defaults` line states that baseline, a key no `set` touches
/// still holds the reset value. Reading that is decoding the format, not
/// inventing a value -- but only while the baseline itself is certain, so the
/// pack must carry a proven reset table and the firmware must be a plain
/// release on the certified line rather than a vendor build.
fn derive_defaults(
    d: &mut ConfigDocument,
    pack: Option<&'static compatibility::Pack>,
    baseline: Option<u32>,
) {
    // Without a certified schema the interface already says so, and the reason
    // is the same one it gives for every other semantic view.
    let Some(pack) = pack.filter(|p| !p.defaults.values.is_empty()) else {
        return;
    };
    let Some(version) = d.firmware.version.clone() else {
        return;
    };
    let Some(line) = baseline else {
        d.derived_note = Some(
            "This backup does not reset the configuration before its settings, so a value it leaves out could be anything the flight controller happened to be holding. Only what the file declares is shown.".into(),
        );
        return;
    };
    if !compatibility::defaults_certified(&version) {
        let note = format!(
            "This build reports {version}, which is not in the verified release list. Values the source omits are left unknown rather than read back from the Betaflight {} defaults, because an unverified or custom build may change any of them and nothing in the backup says whether it did.",
            pack.version
        );
        d.diagnostics.push(Diagnostic {
            line: Some(line),
            severity: "info".into(),
            message: note.clone(),
        });
        d.derived_note = Some(note);
        return;
    }
    let mut count = 0;
    for profile in d.rate_profiles.clone() {
        let scope = Scope::Rate(profile);
        for (key, raw) in &pack.defaults.values {
            let Some(schema) = pack.parameters.get(key) else {
                continue;
            };
            if schema.scope(None, Some(profile)) != scope {
                continue;
            }
            let id = format!("{}:{key}", scope.key());
            // A declared key is never derived, even when its value is invalid:
            // masking a value the schema rejects with the default would report
            // a configuration the source does not contain.
            if d.parameters.contains_key(&id) {
                continue;
            }
            let (value, valid) = schema.parse(raw);
            if !valid {
                continue;
            }
            d.derived.insert(
                id,
                Derived {
                    key: key.clone(),
                    scope: scope.clone(),
                    value,
                    raw_value: raw.clone(),
                    source_version: pack.version.clone(),
                },
            );
            count += 1;
        }
    }
    if count > 0 {
        d.diagnostics.push(Diagnostic{line:Some(line),severity:"info".into(),message:format!(
            "This line resets the configuration, so the {count} rate-profile settings the source never assigns still hold their Betaflight {} defaults. They are shown as read back from the firmware, never as declared, and are not exported.", pack.version)});
    }
}
/// A declared name, or `None` when the backup leaves it unset.
///
/// Betaflight prints an unset string setting as an empty value, and the CLI
/// stores names with the surrounding whitespace stripped, so neither an empty
/// nor a whitespace-only value names anything.
fn declared(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
/// The craft and pilot names the source declares.
///
/// Betaflight 4.4 added `craft_name` and `pilot_name` as settings, so a dump
/// from 4.4 or later carries them as `set` lines. Earlier lines have no such
/// setting and print the craft name only in the `# name:` header the dump
/// writes, which is read as a fallback and never in preference to a declared
/// line. A `defaults` line resets the settings, exactly as it does every other
/// one; it does not erase a header the dump has already printed.
fn declared_names(syntax: &[SyntaxLine]) -> (Option<String>, Option<String>) {
    let (mut craft, mut pilot, mut header) = (None, None, None);
    for line in syntax {
        match &line.command {
            Command::Defaults => {
                craft = None;
                pilot = None;
            }
            Command::Set { key, value } => match key.as_str() {
                "craft_name" => craft = declared(value),
                "pilot_name" => pilot = declared(value),
                _ => {}
            },
            Command::Comment => {
                if let Some(name) = line
                    .raw
                    .trim_start_matches('\u{feff}')
                    .trim_start()
                    .strip_prefix("# name:")
                {
                    header = declared(name);
                }
            }
            _ => {}
        }
    }
    (craft.or(header), pilot)
}
/// The flashed target the source names.
///
/// Betaflight prints `board_name <target>` as a bare command, which the parser
/// keeps as an unsupported line because it identifies the hardware rather than
/// setting anything. The `# config:` comment a dump also carries repeats the
/// target, but at least one real backup truncates it there while the command
/// line holds it in full, so the comment is never read as a substitute. A
/// `defaults` line does not clear it: the board a backup came off is a fact
/// about the file, not a setting the reset restores.
fn declared_board(syntax: &[SyntaxLine]) -> Option<String> {
    let mut board = None;
    for line in syntax {
        if !matches!(line.command, Command::Unsupported) {
            continue;
        }
        let text = line.raw.trim_start_matches('\u{feff}').trim_start();
        if let Some(target) = text
            .strip_prefix("board_name")
            .filter(|target| target.starts_with(char::is_whitespace))
        {
            board = declared(target);
        }
    }
    board
}
pub fn parse_line(raw: &str) -> Result<Command, String> {
    let text = raw.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        return Ok(Command::Blank);
    }
    if text.starts_with('#') {
        return Ok(Command::Comment);
    }
    let words: Vec<_> = text.split_whitespace().collect();
    let bad = || format!("Malformed {} command", words[0]);
    match words[0] {
        "set" => {
            let (key, value) = text[3..].trim().split_once('=').ok_or_else(bad)?;
            let key = key.trim();
            if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(bad());
            }
            Ok(Command::Set {
                key: key.into(),
                value: value.trim().into(),
            })
        }
        "profile" | "rateprofile" => {
            if words.len() != 2 {
                return Err(bad());
            }
            // How many profiles exist is firmware- and build-dependent, so the
            // certified schema decides whether an index is in range (see
            // `analyze`). Rejecting it here would make the selector malformed,
            // which strands every following `set` in an indeterminate scope.
            let index = words[1].parse::<u8>().map_err(|_| bad())?;
            Ok(if words[0] == "profile" {
                Command::Profile { index }
            } else {
                Command::Rateprofile { index }
            })
        }
        "defaults" => {
            if words.len() > 2 || (words.len() == 2 && words[1] != "nosave") {
                return Err(bad());
            }
            Ok(Command::Defaults)
        }
        "feature" => {
            if words.len() != 2 {
                return Err(bad());
            }
            let name = words[1].trim_start_matches('-');
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                return Err(bad());
            }
            Ok(Command::Feature {
                name: name.into(),
                enabled: !words[1].starts_with('-'),
            })
        }
        "serial" => {
            if words.len() != 7 {
                return Err(bad());
            }
            let identifier = words[1]
                .parse()
                .ok()
                .or_else(|| named_port_id(words[1]))
                .ok_or_else(bad)?;
            let v = words[2..]
                .iter()
                .map(|v| v.parse::<u32>().map_err(|_| bad()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Command::Serial {
                identifier,
                mask: v[0],
                baud: [v[1], v[2], v[3], v[4]],
            })
        }
        "aux" => {
            if !(6..=8).contains(&words.len()) {
                return Err(bad());
            }
            let v = words[1..]
                .iter()
                .map(|v| v.parse::<u32>().map_err(|_| bad()))
                .collect::<Result<Vec<_>, _>>()?;
            // `auxChannelIndex` is a byte that `printAux` writes unclamped, so
            // a value above the assignable channels is a state the firmware
            // itself emits. `analyze` reports it; rejecting the line here would
            // drop a mode the user actually has.
            if v[0] > 19
                || v[1] > 255
                || v[2] > 255
                || v[3] < 900
                || v[4] > 2100
                || v[3] > v[4]
                || v[3] > 2100
                || v[4] < 900
                || v[3] % 25 != 0
                || v[4] % 25 != 0
                || v.get(5).is_some_and(|v| *v > 1)
                || v.get(6).is_some_and(|v| *v > 255)
            {
                return Err("Invalid AUX index, channel, range, or logic".into());
            }
            Ok(Command::Aux {
                index: v[0],
                mode: v[1],
                channel: v[2],
                start: v[3],
                end: v[4],
                logic: v.get(5).copied(),
                linked: v.get(6).copied(),
            })
        }
        "vtxtable" | "vtx" | "rxfail" | "rxrange" | "adjrange" => {
            if words.len() < 2 {
                return Err(bad());
            }
            Ok(Command::Collection {
                name: words[0].into(),
                operands: words[1..].iter().map(|s| s.to_string()).collect(),
            })
        }
        "save" | "batch" | "diff" | "dump" => Ok(Command::Device),
        _ => Ok(Command::Unsupported),
    }
}
fn version_from(header: &str) -> Option<String> {
    header
        .split_whitespace()
        .find(|w| {
            let base = w.split(['-', '+']).next().unwrap_or(w);
            let mut parts = base.split('.');
            parts.by_ref().take(3).count() == 3
                && base
                    .split('.')
                    .take(3)
                    .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_owned)
}
pub fn analyze(text: &str, label: &str, source_id: &str) -> Result<Artifact, String> {
    if text.lines().take(100_001).count() > 100_000 {
        return Err("Text imports are limited to 100,000 lines".into());
    }
    if text.len() > source::TEXT_LIMIT {
        return Err("Text imports are limited to 16 MiB".into());
    }
    if text.contains('\0') {
        return Err("Binary data is not a CLI configuration".into());
    }
    if text.trim().is_empty() {
        return Err("Paste or select a non-empty configuration".into());
    }
    let hash = source::hash(text.as_bytes());
    let header = text.lines().find(|l| {
        l.trim_start_matches('\u{feff}')
            .trim()
            .starts_with("# Betaflight /")
    });
    let family = if header.is_some() {
        "betaflight"
    } else if text.lines().any(|l| {
        l.to_ascii_lowercase().contains("# inav/") || l.to_ascii_lowercase().contains("# inav /")
    }) {
        "inav"
    } else if text.contains("ArduPilot")
        || text.contains("ArduCopter")
        || text.lines().any(|l| l.starts_with("ATC_RAT_"))
    {
        "ardupilot"
    } else {
        "unknown"
    };
    if source::is_blackbox(text.as_bytes()) || family == "inav" || family == "ardupilot" {
        let bb = source::is_blackbox(text.as_bytes());
        return Ok(Artifact::Recognized(RecognizedArtifact {
            id: hash,
            title: label.into(),
            family: if bb { "blackbox".into() } else { family.into() },
            message: if bb {
                "Telemetry decoding available in Phase 3".into()
            } else {
                "This firmware adapter is planned for Phase 4. Source remains unchanged.".into()
            },
        }));
    }
    let version = header.and_then(version_from);
    let pack = if family == "betaflight" {
        compatibility::pack(version.as_deref())
    } else {
        None
    };
    let mut d = ConfigDocument {
        id: hash.clone(),
        source_id: source_id.into(),
        title: label.into(),
        hash,
        firmware: Firmware {
            family: family.into(),
            version,
            board_name: None,
            header: header.map(str::to_owned),
            pack_id: pack.map(|p| p.id.clone()),
        },
        craft_name: None,
        pilot_name: None,
        completeness: "partial".into(),
        parameters: BTreeMap::new(),
        derived: BTreeMap::new(),
        derived_note: None,
        syntax: Vec::new(),
        diagnostics: Vec::new(),
        ports: Vec::new(),
        modes: Vec::new(),
        features: BTreeMap::new(),
        pid_profiles: Vec::new(),
        rate_profiles: Vec::new(),
        selected_pid: None,
        selected_rate: None,
    };
    let mut pid = None;
    let mut rate = None;
    let mut baseline = None;
    let mut pids = BTreeSet::new();
    let mut rates = BTreeSet::new();
    let mut offset = 0;
    d.diagnostics.push(Diagnostic{line:None,severity:"info".into(),message:"No uniquely verified target baseline is bundled. Omitted values remain unknown; this is a declared configuration, not a simulation of boot-time corrections.".into()});
    if pack.is_none() {
        d.diagnostics.push(Diagnostic{line:None,severity:"warning".into(),message:"No compatible schema matches this firmware line. Raw inspection is available; semantic plots and export are disabled.".into()});
    }
    for (i, raw) in text.split_inclusive('\n').enumerate() {
        let line = i as u32 + 1;
        let command = match parse_line(raw) {
            Ok(c) => c,
            Err(message) => {
                d.diagnostics.push(Diagnostic {
                    line: Some(line),
                    severity: "error".into(),
                    message,
                });
                Command::Malformed
            }
        };
        match &command {
            Command::Defaults => {
                baseline = Some(line);
                d.parameters.clear();
                d.ports.clear();
                d.modes.clear();
                d.features.clear();
                pids.clear();
                rates.clear();
                pid = Some(0);
                rate = Some(0);
                pids.insert(0);
                rates.insert(0);
            }
            Command::Profile { index } => {
                if let Some(limit) = pack.map(|p| p.profiles.pid) {
                    if *index >= limit {
                        d.diagnostics.push(Diagnostic {
                            line: Some(line),
                            severity: "warning".into(),
                            message: format!(
                                "Profile {index} is outside the {limit} PID profiles certified for this firmware. Its settings are attributed to it exactly as declared."
                            ),
                        });
                    }
                }
                pid = Some(*index);
                pids.insert(*index);
            }
            Command::Rateprofile { index } => {
                if let Some(limit) = pack.map(|p| p.profiles.rate) {
                    if *index >= limit {
                        d.diagnostics.push(Diagnostic {
                            line: Some(line),
                            severity: "warning".into(),
                            message: format!(
                                "Rate profile {index} is outside the {limit} rate profiles certified for this firmware. Its settings are attributed to it exactly as declared."
                            ),
                        });
                    }
                }
                rate = Some(*index);
                rates.insert(*index);
            }
            Command::Set { key, value } => {
                let schema = pack.and_then(|p| p.parameters.get(key));
                let scope = schema.map(|s| s.scope(pid, rate)).unwrap_or(Scope::Unknown);
                let (value_typed, valid) = schema
                    .map(|s| s.parse(value))
                    .unwrap_or((Value::Text(value.clone()), true));
                if !valid {
                    d.diagnostics.push(Diagnostic {
                        line: Some(line),
                        severity: "error".into(),
                        message: format!("{key} has an invalid value for this schema"),
                    });
                }
                if schema.is_some() && scope == Scope::Unknown {
                    d.diagnostics.push(Diagnostic {
                        line: Some(line),
                        severity: "warning".into(),
                        message: format!("{key}: profile selection is unknown"),
                    });
                }
                let mapkey = if scope == Scope::Unknown {
                    format!("unknown:{line}:{key}")
                } else {
                    format!("{}:{key}", scope.key())
                };
                d.parameters.insert(
                    mapkey,
                    Parameter {
                        key: key.clone(),
                        semantic_key: key.clone(),
                        raw_value: value.clone(),
                        value: value_typed,
                        scope,
                        line,
                        valid,
                        supported: schema.is_some(),
                        unit: if key.ends_with("_hz") {
                            Some("Hz".into())
                        } else {
                            None
                        },
                        pack_id: pack.map(|p| p.id.clone()),
                    },
                );
            }
            Command::Feature { name, enabled } => {
                d.features.insert(name.clone(), *enabled);
            }
            Command::Serial {
                identifier,
                mask,
                baud,
            } => {
                let modern = named_serial_version(d.firmware.version.as_deref());
                let identifier = if modern && (0..20).contains(identifier) {
                    *identifier + 51
                } else {
                    *identifier
                };
                d.ports.retain(|p| p.identifier != identifier);
                d.ports.push(Port {
                    identifier,
                    name: port_name(identifier, modern),
                    mask: *mask,
                    functions: port_functions(*mask, modern),
                    baud: *baud,
                    line,
                });
            }
            Command::Aux {
                index,
                mode,
                channel,
                start,
                end,
                logic,
                linked,
            } => {
                let channel_assigned = *channel < AUX_CHANNEL_COUNT;
                if !channel_assigned {
                    d.diagnostics.push(Diagnostic {
                        line: Some(line),
                        severity: "warning".into(),
                        message: format!(
                            "AUX channel {channel} is outside the {AUX_CHANNEL_COUNT} assignable channels, which is what Betaflight stores for a mode with no channel. The range is preserved as declared, but pasting this line back would make the firmware discard the whole mode."
                        ),
                    });
                }
                d.modes.retain(|m| m.index != *index);
                d.modes.push(Mode {
                    index: *index,
                    mode_id: *mode,
                    name: mode_name(*mode),
                    channel: *channel,
                    channel_assigned,
                    start: *start,
                    end: *end,
                    logic: *logic,
                    linked: *linked,
                    line,
                });
            }
            Command::Unsupported | Command::Collection { .. } => d.diagnostics.push(Diagnostic {
                line: Some(line),
                severity: "info".into(),
                message:
                    "Command preserved in source; its semantics are not interpreted or exported."
                        .into(),
            }),
            Command::Malformed => {
                // An invalid selector/reset makes subsequent scope indeterminate.
                let first = raw.split_whitespace().next().unwrap_or("");
                if first == "profile" {
                    pid = None;
                }
                if first == "rateprofile" {
                    rate = None;
                }
                if first == "defaults" {
                    pid = None;
                    rate = None;
                    d.parameters.clear();
                    d.ports.clear();
                    d.modes.clear();
                    d.features.clear();
                }
            }
            _ => {}
        }
        d.syntax.push(SyntaxLine {
            line,
            start: offset,
            end: offset + raw.len() as u32,
            raw: raw.into(),
            command,
        });
        offset += raw.len() as u32;
    }
    d.pid_profiles = pids.into_iter().collect();
    d.rate_profiles = rates.into_iter().collect();
    d.selected_pid = pid;
    d.selected_rate = rate;
    (d.craft_name, d.pilot_name) = declared_names(&d.syntax);
    d.firmware.board_name = declared_board(&d.syntax);
    derive_defaults(&mut d, pack, baseline);
    Ok(Artifact::Config(Box::new(d)))
}
pub(crate) fn named_serial_version(version: Option<&str>) -> bool {
    version.is_some_and(|v| v.starts_with("2025.12."))
}

pub(crate) fn serial_port_token(id: i32, modern: bool) -> Option<String> {
    match id {
        50..=60 if modern => Some(format!("UART{}", id - 50)),
        70..=79 if modern => Some(format!("PIOUART{}", id - 70)),
        20 if modern => Some("VCP".into()),
        30..=31 if modern => Some(format!("SOFT{}", id - 29)),
        40 if modern => Some("LPUART1".into()),
        0..=9 | 20 | 30..=31 | 40 if !modern => Some(id.to_string()),
        _ => None,
    }
}

fn named_port_id(token: &str) -> Option<i32> {
    (20..=79).find(|id| {
        serial_port_token(*id, true).is_some_and(|name| name.eq_ignore_ascii_case(token))
    })
}

fn port_name(id: i32, modern: bool) -> String {
    if modern {
        match id {
            50..=60 => return format!("UART {}", id - 50),
            70..=79 => return format!("PIO UART {}", id - 70),
            0..=19 => return format!("Unknown port {id}"),
            _ => {}
        }
    }
    match id {
        0..=9 => format!("UART {}", id + 1),
        20 => "USB VCP".into(),
        30..=31 => format!("Softserial {}", id - 29),
        40 => "LPUART 1".into(),
        _ => format!("Unknown port {id}"),
    }
}
fn port_functions(mask: u32, modern: bool) -> Vec<String> {
    let names = [
        (0, "MSP"),
        (1, "GPS"),
        (2, "FrSky Hub"),
        (3, "HoTT"),
        (4, "LTM"),
        (5, "SmartPort"),
        (6, "Serial RX"),
        (7, "Blackbox"),
        (9, "MAVLink"),
        (10, "ESC sensor"),
        (11, "SmartAudio"),
        (12, "iBus"),
        (13, "Tramp"),
        (14, "RC device"),
        (15, "Lidar TF"),
        (16, "FrSky OSD"),
        (17, "VTX MSP"),
    ];
    let mut out = Vec::new();
    let mut known = 0;
    for (bit, name) in names {
        known |= 1 << bit;
        if mask & (1 << bit) != 0 {
            out.push(name.into());
        }
    }
    if modern {
        known |= 1 << 18;
        if mask & (1 << 18) != 0 {
            out.push("Gimbal".into());
        }
    }
    if mask & !known != 0 {
        out.push(format!("Unknown bits 0x{:x}", mask & !known));
    }
    out
}
fn mode_name(id: u32) -> String {
    match id {
        0 => "ARM",
        1 => "ANGLE",
        2 => "HORIZON",
        4 => "ANTI GRAVITY",
        5 => "MAG",
        6 => "HEADFREE",
        7 => "HEADADJ",
        8 => "CAMSTAB",
        13 => "BEEPER",
        19 => "OSD",
        20 => "TELEMETRY",
        26 => "BLACKBOX",
        27 => "FAILSAFE",
        28 => "AIR MODE",
        30 => "FPV ANGLE MIX",
        31 => "BLACKBOX ERASE",
        35 => "FLIP OVER AFTER CRASH",
        36 => "PREARM",
        46 => "GPS RESCUE",
        _ => return format!("Mode ID {id}"),
    }
    .into()
}

#[cfg(test)]
mod serial_tests {
    use super::*;

    #[test]
    fn serial_export_tokens_round_trip_without_aliasing_uart_zero() {
        for id in [20, 30, 31, 40, 50, 51, 60, 70, 79] {
            let token = serial_port_token(id, true).unwrap();
            let text =
                format!("# Betaflight / STM32F405 2025.12.3\nserial {token} 1 115200 0 0 0\n");
            let Artifact::Config(d) = analyze(&text, "test", "test").unwrap() else {
                panic!()
            };
            assert_eq!(d.ports[0].identifier, id);
        }
        assert_eq!(serial_port_token(0, false).as_deref(), Some("0"));
        assert_eq!(serial_port_token(50, true).as_deref(), Some("UART0"));
        assert!(serial_port_token(50, false).is_none());
        assert!(serial_port_token(0, true).is_none());
    }
}
