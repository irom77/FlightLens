//! Build-invariant PID defaults, independently certified from rate defaults.
use crate::{compatibility::Pack, model::*};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Defaults {
    source_version: String,
    verified: Vec<String>,
    values: BTreeMap<String, String>,
}

pub(crate) fn derive(d: &mut ConfigDocument, pack: Option<&Pack>, baseline: Option<u32>) {
    static TABLE: OnceLock<Defaults> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../compatibility/pid-defaults.json"))
            .expect("bundled PID defaults must validate")
    });
    let (Some(pack), Some(line), Some(version)) = (pack, baseline, &d.firmware.version) else {
        return;
    };
    if !table.verified.contains(version) {
        return;
    }
    // Only the final valid reset supplies a baseline. Do not try to replay
    // tuning arithmetic, ambiguous selectors/assignments, or malformed resets.
    let uncertain = d
        .syntax
        .iter()
        .filter(|s| s.line > line)
        .find(|s| match &s.command {
            Command::Profile { index } => *index >= pack.profiles.pid,
            Command::Unsupported | Command::Malformed => {
                let mut words = s.raw.trim_start_matches('\u{feff}').split_whitespace();
                let first = words.next().unwrap_or("");
                (first.eq_ignore_ascii_case("simplified_tuning")
                    && !(words
                        .next()
                        .is_some_and(|w| w.eq_ignore_ascii_case("disable"))
                        && words.next().is_none()))
                    || ["defaults", "profile", "set"]
                        .iter()
                        .any(|name| first.eq_ignore_ascii_case(name))
            }
            _ => false,
        });
    if let Some(s) = uncertain {
        d.diagnostics.push(Diagnostic {
            line: Some(s.line),
            severity: "info".into(),
            message: "Omitted PID gains remain unknown after an unmodeled tuning command or ambiguous reset, profile or assignment. A later valid defaults command can establish a new baseline. Declared settings are preserved, not a simulation of command effects.".into(),
        });
        return;
    }
    let mut count = 0;
    for profile in &d.pid_profiles {
        if *profile >= pack.profiles.pid {
            continue;
        }
        let scope = Scope::Pid(*profile);
        for (key, raw) in &table.values {
            let id = format!("{}:{key}", scope.key());
            // Even an invalid explicit value must block recovery. An assignment
            // with unknown scope could have targeted any profile.
            if d.parameters.contains_key(&id)
                || d.parameters
                    .values()
                    .any(|p| p.key == *key && p.scope == Scope::Unknown)
            {
                continue;
            }
            let Some(schema) = pack.parameters.get(key) else {
                continue;
            };
            let (value, valid) = schema.parse(raw);
            if !valid || schema.scope(Some(*profile), None) != scope {
                continue;
            }
            d.derived.insert(
                id,
                Derived {
                    key: key.clone(),
                    scope: scope.clone(),
                    value,
                    raw_value: raw.clone(),
                    source_version: table.source_version.clone(),
                },
            );
            count += 1;
        }
    }
    if count > 0 {
        d.diagnostics.push(Diagnostic {
            line: Some(line), severity: "info".into(),
            message: format!("{count} omitted PID gains read back from verified Betaflight {} defaults. Roll/pitch D and D-min/D-max are not recovered because their build conditions are not certified. Recovered values are not declared or exported.", table.source_version),
        });
    }
}
