//! Bounded, structured inputs. Never serialize a ConfigDocument into a prompt.
use crate::{
    analysis::{self, Curve, Point},
    compatibility,
    feedback::sensitivity,
    ConfigDocument, Scope,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

pub const MAX_DIFF_ROWS: usize = 400;
pub const MAX_FIELD_CHARS: usize = 512;
const MARKER: &str = "… [truncated]";

fn bounded(value: &str) -> String {
    if value.chars().count() <= MAX_FIELD_CHARS {
        return value.into();
    }
    value
        .chars()
        .take(MAX_FIELD_CHARS - MARKER.chars().count())
        .collect::<String>()
        + MARKER
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DigestFirmware {
    pub family: String,
    pub version: Option<String>,
    pub pack_id: Option<String>,
    pub has_dump_all: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Declared,
    Derived,
    Unknown,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DigestParameter {
    pub semantic_key: String,
    // None is unknown, including invalid declarations. Never an implied zero.
    pub value: Option<String>,
    pub unit: Option<String>,
    pub scope: Scope,
    pub provenance: Provenance,
    pub supported: bool,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DigestCurve {
    pub name: String,
    pub available: bool,
    pub samples: Vec<Point>,
    pub reason: Option<String>,
    pub derived_inputs: Vec<String>,
}
fn curve(c: Curve) -> DigestCurve {
    let samples = if c.points.is_empty() {
        vec![]
    } else {
        [
            0,
            c.points.len() / 4,
            c.points.len() / 2,
            c.points.len() * 3 / 4,
            c.points.len() - 1,
        ]
        .into_iter()
        .map(|i| c.points[i].clone())
        .collect()
    };
    DigestCurve {
        name: c.name,
        available: !samples.is_empty(),
        samples,
        reason: c.reason,
        derived_inputs: c.derived_inputs,
    }
}
#[derive(Clone, Serialize, Deserialize, TS)]
pub struct DigestAudit {
    pub id: String,
    pub status: String,
    pub severity: String,
    pub explanation: String,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InspectorDigest {
    pub label: String,
    pub firmware: DigestFirmware,
    pub rate_profile: u8,
    pub pid_profile: u8,
    pub parameters: Vec<DigestParameter>,
    pub rates: Vec<DigestCurve>,
    pub throttle: DigestCurve,
    pub audits: Vec<DigestAudit>,
    pub diagnostics: BTreeMap<String, usize>,
}
impl InspectorDigest {
    pub fn build(document: &ConfigDocument, rate_profile: u8, pid_profile: u8) -> Self {
        // Enumerating the schema bounds cardinality and excludes vendor-only keys.
        // Only the parser's certified recovery is used; never derive defaults here.
        let pack = compatibility::pack(document.firmware.version.as_deref())
            .filter(|p| document.firmware.pack_id.as_deref() == Some(p.id.as_str()));
        let parameters = pack
            .into_iter()
            .flat_map(|p| &p.parameters)
            .filter_map(|(key, schema)| {
                if sensitivity(key).is_some() {
                    return None;
                }
                let scope = schema.scope(Some(pid_profile), Some(rate_profile));
                let declared = document.parameters.get(&format!("{}:{key}", scope.key()));
                let derived = document.derived_value(&scope, key);
                let (value, provenance) = match declared {
                    Some(p) if p.valid && p.supported => {
                        (Some(bounded(&p.value.cli())), Provenance::Declared)
                    }
                    Some(_) => (None, Provenance::Unknown),
                    None => match derived {
                        Some(p) => (Some(bounded(&p.value.cli())), Provenance::Derived),
                        None => (None, Provenance::Unknown),
                    },
                };
                Some(DigestParameter {
                    semantic_key: key.clone(),
                    value,
                    unit: key.ends_with("_hz").then(|| "Hz".into()),
                    scope,
                    provenance,
                    supported: true,
                })
            })
            .collect();
        let inspection = analysis::inspect(document, rate_profile);
        let mut diagnostics = BTreeMap::from([
            ("error".into(), 0),
            ("warning".into(), 0),
            ("info".into(), 0),
            ("other".into(), 0),
        ]);
        for diagnostic in &document.diagnostics {
            let key = match diagnostic.severity.as_str() {
                "error" => "error",
                "warning" => "warning",
                "info" => "info",
                _ => "other",
            };
            *diagnostics.get_mut(key).expect("fixed severity") += 1;
        }
        Self {
            label: "Backup A".into(),
            firmware: DigestFirmware {
                family: bounded(&document.firmware.family),
                version: document.firmware.version.as_deref().map(bounded),
                pack_id: pack.map(|p| p.id.clone()),
                has_dump_all: document.source_evidence().has_dump_all,
            },
            rate_profile,
            pid_profile,
            parameters,
            rates: inspection.rates.into_iter().map(curve).collect(),
            throttle: curve(inspection.throttle),
            audits: inspection
                .audits
                .into_iter()
                .map(|a| DigestAudit {
                    id: a.id,
                    status: a.status,
                    severity: a.severity,
                    explanation: a.explanation,
                })
                .collect(),
            diagnostics,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffRowInput {
    pub section: String,
    pub key: String,
    pub scope: String,
    pub status: String,
    pub values: Vec<Option<String>>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct Truncation {
    pub omitted: usize,
    pub sections: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiffDigest {
    pub labels: Vec<String>,
    pub baseline: Option<usize>,
    pub rows: Vec<DiffRowInput>,
    pub truncated: Option<Truncation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestError {
    InvalidSlots,
    InvalidBaseline,
    SensitiveKey,
    InvalidSection,
    InvalidStatus,
    InvalidValues,
}
impl std::fmt::Display for DigestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidSlots => "Select two or three backups.",
            Self::InvalidBaseline => "The baseline must refer to a selected backup.",
            Self::SensitiveKey => {
                "The comparison contains a sensitive setting; remove it before preparing a summary."
            }
            Self::InvalidSection => "The comparison contains an unsupported section.",
            Self::InvalidStatus => "The comparison contains an unsupported status.",
            Self::InvalidValues => "Each comparison row must contain one value per backup.",
        })
    }
}
impl std::error::Error for DigestError {}
impl DiffDigest {
    /// Accept only the slot count, never filenames or a local label mapping.
    pub fn build(
        slots: usize,
        baseline: Option<usize>,
        rows: &[DiffRowInput],
    ) -> Result<Self, DigestError> {
        if !(2..=3).contains(&slots) {
            return Err(DigestError::InvalidSlots);
        }
        if baseline.is_some_and(|b| b >= slots) {
            return Err(DigestError::InvalidBaseline);
        }
        let mut output = Vec::new();
        let mut omitted_sections = BTreeSet::new();
        for (index, row) in rows.iter().enumerate() {
            if sensitivity(&row.key).is_some() {
                return Err(DigestError::SensitiveKey);
            }
            if ![
                "parameters",
                "features",
                "ports",
                "modes",
                "rxrange",
                "rxfail",
                "vtx",
                "adjrange",
                "collections",
            ]
            .contains(&row.section.as_str())
            {
                return Err(DigestError::InvalidSection);
            }
            if ![
                "changed",
                "one_sided",
                "conflict",
                "unknown",
                "not_comparable",
            ]
            .contains(&row.status.as_str())
            {
                return Err(DigestError::InvalidStatus);
            }
            if row.values.len() != slots {
                return Err(DigestError::InvalidValues);
            }
            if index >= MAX_DIFF_ROWS {
                omitted_sections.insert(row.section.clone());
                continue;
            }
            output.push(DiffRowInput {
                section: row.section.clone(),
                key: bounded(&row.key),
                scope: bounded(&row.scope),
                status: row.status.clone(),
                values: row
                    .values
                    .iter()
                    .map(|v| v.as_deref().map(bounded))
                    .collect(),
                reason: row.reason.as_deref().map(bounded),
            });
        }
        Ok(Self {
            labels: ["Backup A", "Backup B", "Backup C"][..slots]
                .iter()
                .map(|s| (*s).into())
                .collect(),
            baseline,
            rows: output,
            truncated: (rows.len() > MAX_DIFF_ROWS).then(|| Truncation {
                omitted: rows.len() - MAX_DIFF_ROWS,
                sections: omitted_sections.into_iter().collect(),
            }),
        })
    }
}
