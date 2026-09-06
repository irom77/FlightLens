use crate::{compatibility, model::*, source};
use std::collections::{BTreeMap, BTreeSet};

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
            let index = words[1].parse::<u8>().map_err(|_| bad())?;
            if index > 5 || (words[0] == "profile" && index > 2) {
                return Err("Profile index is outside supported Betaflight bounds".into());
            }
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
            let identifier = words[1].parse().map_err(|_| bad())?;
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
            if v[0] > 19
                || v[1] > 255
                || v[2] > 13
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
            let p: Vec<_> = w.split('.').collect();
            p.len() == 3
                && p.iter()
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
                "Telemetry decoding available in Phase 4".into()
            } else {
                "This firmware adapter is planned for Phase 3. Source remains unchanged.".into()
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
            header: header.map(str::to_owned),
            pack_id: pack.map(|p| p.id.clone()),
        },
        completeness: "partial".into(),
        parameters: BTreeMap::new(),
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
    let mut pids = BTreeSet::new();
    let mut rates = BTreeSet::new();
    let mut offset = 0;
    d.diagnostics.push(Diagnostic{line:None,severity:"info".into(),message:"No uniquely verified target baseline is bundled. Omitted values remain unknown; this is a declared configuration, not a simulation of boot-time corrections.".into()});
    if pack.is_none() {
        d.diagnostics.push(Diagnostic{line:None,severity:"warning".into(),message:"No exact compatibility pack matches this header. Raw inspection is available; semantic plots and export are disabled.".into()});
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
                pid = Some(*index);
                pids.insert(*index);
            }
            Command::Rateprofile { index } => {
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
                d.ports.retain(|p| p.identifier != *identifier);
                d.ports.push(Port {
                    identifier: *identifier,
                    name: port_name(*identifier),
                    mask: *mask,
                    functions: port_functions(*mask),
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
                d.modes.retain(|m| m.index != *index);
                d.modes.push(Mode {
                    index: *index,
                    mode_id: *mode,
                    name: mode_name(*mode),
                    channel: *channel,
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
    Ok(Artifact::Config(Box::new(d)))
}
fn port_name(id: i32) -> String {
    match id {
        0..=9 => format!("UART {}", id + 1),
        20 => "USB VCP".into(),
        30..=31 => format!("Softserial {}", id - 29),
        40 => "LPUART 1".into(),
        _ => format!("Unknown port {id}"),
    }
}
fn port_functions(mask: u32) -> Vec<String> {
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
