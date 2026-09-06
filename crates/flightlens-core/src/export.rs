use crate::{compatibility, model::*, parser::analyze};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub groups: Vec<String>,
    pub pid_profile: u8,
    pub rate_profile: u8,
    pub destination_header: String,
    pub include_save: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedSnippet {
    pub text: String,
    pub warnings: Vec<String>,
    pub additions: Vec<String>,
}
fn in_group(key: &str, scope: &Scope, group: &str) -> bool {
    match group {
        "rates" => matches!(scope, Scope::Rate(_)),
        "pids_filters" => {
            matches!(scope, Scope::Pid(_))
                || key.starts_with("gyro_lpf")
                || key.starts_with("gyro_lowpass")
                || key.starts_with("dyn_notch_")
                || key.starts_with("rpm_filter_")
        }
        "vtx" => key.starts_with("vtx_"),
        "osd" => {
            key.starts_with("osd_")
                || key.starts_with("vcd_")
                || key == "displayport_msp_serial"
                || key.starts_with("displayport_msp_")
        }
        _ => false,
    }
}
pub fn export(d: &ConfigDocument, r: &ExportRequest) -> Result<ValidatedSnippet, String> {
    let pack = compatibility::pack(d.firmware.version.as_deref())
        .filter(|_| d.firmware.family == "betaflight")
        .ok_or("An exact compatibility pack is required")?;
    if d.firmware.header.as_deref() != Some(r.destination_header.as_str()) {
        return Err("Export currently requires the exact source firmware/target header; cross-target semantics are not certified".into());
    }
    if r.groups.is_empty() {
        return Err("Select at least one group".into());
    }
    if r.groups.iter().any(|g| {
        !matches!(
            g.as_str(),
            "rates" | "pids_filters" | "modes" | "osd" | "serial" | "vtx"
        )
    }) {
        return Err(
            "Unsupported export group; VTX table dependency validation is not yet certified".into(),
        );
    }
    if d.diagnostics.iter().any(|diag| diag.severity == "error") {
        return Err("Resolve malformed or invalid source settings before exporting".into());
    }
    let has = |g: &str| r.groups.iter().any(|x| x == g);
    let mut selected: Vec<_> = d
        .parameters
        .values()
        .filter(|p| {
            r.groups.iter().any(|g| in_group(&p.key, &p.scope, g))
                && match p.scope {
                    Scope::Pid(n) => n == r.pid_profile,
                    Scope::Rate(n) => n == r.rate_profile,
                    _ => true,
                }
        })
        .collect();
    let mut additions = Vec::new();
    if has("rates") {
        for key in [
            "rates_type",
            "roll_rc_rate",
            "roll_srate",
            "roll_expo",
            "pitch_rc_rate",
            "pitch_srate",
            "pitch_expo",
            "yaw_rc_rate",
            "yaw_srate",
            "yaw_expo",
        ] {
            if d.parameter(&Scope::Rate(r.rate_profile), key).is_none() {
                return Err(format!(
                    "Required rate dependency {key} is unknown or invalid"
                ));
            }
        }
        if d.text(&Scope::Rate(r.rate_profile), "rates_type")
            .as_deref()
            == Some("QUICK")
            && d.parameter(&Scope::Rate(r.rate_profile), "quickrates_rc_expo")
                .is_none()
        {
            return Err("quickrates_rc_expo is required for QuickRates".into());
        }
        additions.push(format!(
            "Rate type and rateprofile {} selector",
            r.rate_profile
        ));
    }
    if has("pids_filters") && !selected.iter().any(|p| matches!(p.scope, Scope::Pid(_))) {
        return Err("Selected PID profile has no known settings".into());
    }
    if has("osd") && !selected.iter().any(|p| p.key.starts_with("osd_")) {
        return Err("No known OSD settings to export".into());
    }
    // Avoid raw/unvalidated settings, even when other members of the group are valid.
    for p in &selected {
        let schema = pack
            .parameters
            .get(&p.key)
            .ok_or("Unknown export setting")?;
        if !p.valid || !p.supported || p.scope == Scope::Unknown || !schema.exportable() {
            return Err(format!(
                "{} does not have certified export validation",
                p.key
            ));
        }
    }
    selected.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.key.cmp(&b.key)));
    let mut lines = vec![
        r.destination_header.clone(),
        "# FlightLens selected settings; review before applying".into(),
    ];
    if has("vtx") {
        if d.syntax
            .iter()
            .rev()
            .take_while(|l| !matches!(l.command, Command::Defaults))
            .any(|l| matches!(&l.command, Command::Collection{name,..} if name=="vtx"))
        {
            return Err(
                "VTX AUX ranges need independent dependency validation; export is blocked".into(),
            );
        }
        let table = crate::vtx::Table::from_config(d)?;
        table.validate_selection(d)?;
        lines.extend(table.lines()?);
        additions.push("Complete explicit VTX frequency and power table".into());
    }
    let mut scope = Scope::Global;
    for p in &selected {
        if p.scope != scope {
            match p.scope {
                Scope::Pid(n) => lines.push(format!("profile {n}")),
                Scope::Rate(n) => lines.push(format!("rateprofile {n}")),
                _ => {}
            }
            scope = p.scope.clone();
        }
        lines.push(format!("set {} = {}", p.key, p.raw_value));
    }
    if has("modes") {
        if d.modes.is_empty() {
            return Err("No explicit mode ranges to export".into());
        }
        for m in &d.modes {
            lines.push(format!(
                "aux {} {} {} {} {}{}{}",
                m.index,
                m.mode_id,
                m.channel,
                m.start,
                m.end,
                m.logic.map(|v| format!(" {v}")).unwrap_or_default(),
                m.linked.map(|v| format!(" {v}")).unwrap_or_default()
            ));
        }
    }
    if has("serial") {
        if d.ports.is_empty() {
            return Err("No explicit serial allocations to export".into());
        }
        let baud = [
            0, 9600, 19200, 38400, 57600, 115200, 230400, 250000, 400000, 460800, 500000, 921600,
            1000000, 1500000, 2000000, 2470000,
        ];
        for p in &d.ports {
            if p.name.starts_with("Unknown")
                || p.functions.iter().any(|s| s.starts_with("Unknown"))
                || !p.baud.iter().all(|b| baud.contains(b))
            {
                return Err(
                    "Serial allocation includes unsupported identifiers, masks, or baud rates"
                        .into(),
                );
            }
            lines.push(format!(
                "serial {} {} {} {} {} {}",
                p.identifier, p.mask, p.baud[0], p.baud[1], p.baud[2], p.baud[3]
            ));
        }
    }
    for (name, enabled) in &d.features {
        if (has("osd") && name == "OSD")
            || (has("serial")
                && matches!(
                    name.as_str(),
                    "RX_SERIAL" | "SOFTSERIAL" | "TELEMETRY" | "GPS"
                ))
        {
            lines.push(format!("feature {}{name}", if *enabled { "" } else { "-" }));
            additions.push(format!("Explicit {name} feature state"));
        }
    }
    if has("rates") {
        let n = d
            .selected_rate
            .ok_or("Original rate profile is unknown; cannot restore selection")?;
        lines.push(format!("rateprofile {n}"));
    }
    if has("pids_filters") {
        let n = d
            .selected_pid
            .ok_or("Original PID profile is unknown; cannot restore selection")?;
        lines.push(format!("profile {n}"));
    }
    if r.include_save {
        lines.push("save".into());
    }
    let text = lines.join("\n") + "\n";
    let Artifact::Config(parsed) = analyze(&text, "Export validation", "export")? else {
        return Err("Export did not parse as a configuration".into());
    };
    if parsed.diagnostics.iter().any(|d| d.severity == "error") {
        return Err("Generated snippet failed parsing".into());
    }
    for p in &selected {
        if parsed
            .parameter(&p.scope, &p.key)
            .is_none_or(|x| x.value != p.value)
        {
            return Err(format!("Semantic round trip failed for {}", p.key));
        }
    }
    if has("modes")
        && serde_json::to_value(
            parsed
                .modes
                .iter()
                .map(|m| {
                    (
                        m.index, m.mode_id, m.channel, m.start, m.end, m.logic, m.linked,
                    )
                })
                .collect::<Vec<_>>(),
        )
        .ok()
            != serde_json::to_value(
                d.modes
                    .iter()
                    .map(|m| {
                        (
                            m.index, m.mode_id, m.channel, m.start, m.end, m.logic, m.linked,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .ok()
    {
        return Err("Mode round trip failed".into());
    }
    if has("serial")
        && parsed
            .ports
            .iter()
            .map(|p| (p.identifier, p.mask, p.baud))
            .collect::<BTreeSet<_>>()
            != d.ports
                .iter()
                .map(|p| (p.identifier, p.mask, p.baud))
                .collect::<BTreeSet<_>>()
    {
        return Err("Serial round trip failed".into());
    }
    if has("vtx") && crate::vtx::Table::from_config(d)? != crate::vtx::Table::from_config(&parsed)?
    {
        return Err("VTX table round trip failed".into());
    }
    Ok(ValidatedSnippet{text,warnings:vec!["Applies explicit selected settings only. Omitted donor values and unknown destination settings are not reset.".into(),"Mode and serial slots are updated by index; other destination entries remain. Review destination allocations before applying.".into(),"Hardware, build features, and boot-time validation are not simulated. No controller connection is made.".into()],additions})
}
