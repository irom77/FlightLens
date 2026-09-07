use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    Integer(i32),
    Decimal(f64),
    Text(String),
}
impl Value {
    pub fn cli(&self) -> String {
        match self {
            Self::Integer(v) => v.to_string(),
            Self::Decimal(v) => v.to_string(),
            Self::Text(v) => v.clone(),
        }
    }
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Integer(v) => Some(*v as f64),
            Self::Decimal(v) => Some(*v),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
pub enum Scope {
    Global,
    Pid(u8),
    Rate(u8),
    Unknown,
}
impl Scope {
    pub fn key(&self) -> String {
        match self {
            Self::Global => "global".into(),
            Self::Pid(n) => format!("pid:{n}"),
            Self::Rate(n) => format!("rate:{n}"),
            Self::Unknown => "unknown".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub key: String,
    pub semantic_key: String,
    pub value: Value,
    pub raw_value: String,
    pub scope: Scope,
    pub line: u32,
    pub valid: bool,
    pub supported: bool,
    pub unit: Option<String>,
    pub pack_id: Option<String>,
}
/// A value no source line declares, read back from the certified pack's reset
/// table. A Betaflight dump prints only what differs from that reset, so on a
/// document that declares `defaults` an omitted key provably still holds it.
///
/// Kept apart from `Parameter` on purpose: a derived value has no source line,
/// must never be presented as a declared one, and is never exported.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Derived {
    pub key: String,
    pub scope: Scope,
    pub value: Value,
    pub raw_value: String,
    /// The certified firmware line whose reset table supplied the value, which
    /// is what stands in for a source line in the interface.
    pub source_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub line: Option<u32>,
    pub severity: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    Set {
        key: String,
        value: String,
    },
    Profile {
        index: u8,
    },
    Rateprofile {
        index: u8,
    },
    Defaults,
    Feature {
        name: String,
        enabled: bool,
    },
    Serial {
        identifier: i32,
        mask: u32,
        baud: [u32; 4],
    },
    Aux {
        index: u32,
        mode: u32,
        channel: u32,
        start: u32,
        end: u32,
        logic: Option<u32>,
        linked: Option<u32>,
    },
    Collection {
        name: String,
        operands: Vec<String>,
    },
    Device,
    Comment,
    Blank,
    Unsupported,
    Malformed,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxLine {
    pub line: u32,
    pub start: u32,
    pub end: u32,
    pub raw: String,
    pub command: Command,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    pub family: String,
    pub version: Option<String>,
    pub header: Option<String>,
    pub pack_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub identifier: i32,
    pub name: String,
    pub mask: u32,
    pub functions: Vec<String>,
    pub baud: [u32; 4],
    pub line: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub index: u32,
    pub mode_id: u32,
    pub name: String,
    pub channel: u32,
    /// Whether `channel` names one of the AUX channels Betaflight can assign.
    /// The firmware prints the stored byte unclamped, so a dump can carry a
    /// channel no build accepts; see `parser::AUX_CHANNEL_COUNT`.
    pub channel_assigned: bool,
    pub start: u32,
    pub end: u32,
    pub logic: Option<u32>,
    pub linked: Option<u32>,
    pub line: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDocument {
    pub id: String,
    pub source_id: String,
    pub title: String,
    pub hash: String,
    pub firmware: Firmware,
    pub completeness: String,
    pub parameters: BTreeMap<String, Parameter>,
    /// Keyed like `parameters`, and disjoint from it: a key any source line
    /// declares is never derived, so an invalid declared value stays unknown
    /// rather than being quietly replaced by the default.
    pub derived: BTreeMap<String, Derived>,
    pub syntax: Vec<SyntaxLine>,
    pub diagnostics: Vec<Diagnostic>,
    pub ports: Vec<Port>,
    pub modes: Vec<Mode>,
    pub features: BTreeMap<String, bool>,
    pub pid_profiles: Vec<u8>,
    pub rate_profiles: Vec<u8>,
    pub selected_pid: Option<u8>,
    pub selected_rate: Option<u8>,
}
impl ConfigDocument {
    pub fn parameter(&self, scope: &Scope, key: &str) -> Option<&Parameter> {
        self.parameters
            .get(&format!("{}:{key}", scope.key()))
            .filter(|p| p.valid && p.supported)
    }
    pub fn number(&self, scope: &Scope, key: &str) -> Option<f64> {
        self.parameter(scope, key)?.value.number()
    }
    pub fn text(&self, scope: &Scope, key: &str) -> Option<String> {
        Some(self.parameter(scope, key)?.value.cli())
    }
    pub fn derived_value(&self, scope: &Scope, key: &str) -> Option<&Derived> {
        self.derived.get(&format!("{}:{key}", scope.key()))
    }
    /// A declared value, or the firmware default where the source omits the key
    /// entirely. Callers that must distinguish the two ask `derived_value`.
    pub fn number_or_default(&self, scope: &Scope, key: &str) -> Option<f64> {
        self.number(scope, key)
            .or_else(|| self.derived_value(scope, key)?.value.number())
    }
    pub fn text_or_default(&self, scope: &Scope, key: &str) -> Option<String> {
        self.text(scope, key)
            .or_else(|| Some(self.derived_value(scope, key)?.value.cli()))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "document", rename_all = "snake_case")]
pub enum Artifact {
    Config(Box<ConfigDocument>),
    Recognized(RecognizedArtifact),
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RecognizedArtifact {
    pub id: String,
    pub title: String,
    pub family: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SourceDescriptor {
    pub id: String,
    pub label: String,
}
