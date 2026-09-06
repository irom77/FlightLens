use crate::model::{Scope, Value};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};
#[derive(Debug, Deserialize)]
pub struct Schema {
    pub scope: String,
    pub kind: String,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub values: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct Pack {
    pub id: String,
    pub version: String,
    pub parameters: BTreeMap<String, Schema>,
}
pub fn packs() -> &'static [Pack; 3] {
    static PACKS: OnceLock<[Pack; 3]> = OnceLock::new();
    PACKS.get_or_init(|| {
        [
            include_str!("../compatibility/betaflight-4.3.0.json"),
            include_str!("../compatibility/betaflight-4.4.0.json"),
            include_str!("../compatibility/betaflight-4.5.0.json"),
        ]
        .map(|p| serde_json::from_str(p).expect("bundled schema must validate"))
    })
}
pub fn pack(version: Option<&str>) -> Option<&'static Pack> {
    packs().iter().find(|p| Some(p.version.as_str()) == version)
}
impl Schema {
    pub fn scope(&self, pid: Option<u8>, rate: Option<u8>) -> Scope {
        match self.scope.as_str() {
            "pid" => pid.map(Scope::Pid).unwrap_or(Scope::Unknown),
            "rate" => rate.map(Scope::Rate).unwrap_or(Scope::Unknown),
            _ => Scope::Global,
        }
    }
    pub fn parse(&self, raw: &str) -> (Value, bool) {
        match self.kind.as_str() {
            "integer" => match raw.parse::<i32>() {
                Ok(n) => (
                    Value::Integer(n),
                    self.min.is_none_or(|min| n >= min) && self.max.is_none_or(|max| n <= max),
                ),
                Err(_) => (Value::Text(raw.into()), false),
            },
            "enum" => (
                Value::Text(raw.into()),
                self.values.is_empty() || self.values.iter().any(|s| s == raw),
            ),
            _ => (Value::Text(raw.into()), !raw.contains(['\r', '\n', '\0'])),
        }
    }
    pub fn exportable(&self) -> bool {
        match self.kind.as_str() {
            // A schema may intentionally leave one bound open when the
            // firmware's limit is build-dependent. The parser still proves
            // that the source value is an integer; requiring both bounds
            // would make otherwise explicit PID feedforward values
            // impossible to export from older Betaflight packs.
            "integer" => self.min.is_some(),
            "enum" => !self.values.is_empty(),
            "string" => true,
            _ => false,
        }
    }
}
