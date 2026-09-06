//! Conservative reconstruction of an explicit VTX table. Never supplies hardware defaults.
use crate::{Command, ConfigDocument};
use std::collections::BTreeMap;
#[derive(Default, Debug, PartialEq)]
pub struct Table {
    bands: Option<usize>,
    channels: Option<usize>,
    levels: Option<usize>,
    rows: BTreeMap<usize, Vec<String>>,
    values: Option<Vec<String>>,
    labels: Option<Vec<String>>,
}
fn count(v: Option<&String>) -> Result<usize, String> {
    v.and_then(|v| v.parse().ok())
        .filter(|n| *n <= 8)
        .ok_or("VTX table count must be 0–8".into())
}
impl Table {
    fn apply(&mut self, v: &[String]) -> Result<(), String> {
        match v.first().map(String::as_str) {
            Some("bands") if v.len() == 2 => {
                let n = count(v.get(1))?;
                self.rows.retain(|i, _| *i <= n);
                self.bands = Some(n);
            }
            Some("channels") if v.len() == 2 => {
                let n = count(v.get(1))?;
                if self.channels != Some(n) {
                    self.rows.clear();
                }
                self.channels = Some(n);
            }
            Some("powerlevels") if v.len() == 2 => {
                let n = count(v.get(1))?;
                if self.levels != Some(n) {
                    self.values = None;
                    self.labels = None;
                }
                self.levels = Some(n);
            }
            Some("powervalues") => {
                let n = self.levels.ok_or("VTX power level count is unknown")?;
                if v.len() != n + 1 || !v[1..].iter().all(|v| v.parse::<u16>().is_ok()) {
                    return Err("Invalid VTX power values".into());
                }
                self.values = Some(v[1..].to_vec());
            }
            Some("powerlabels") => {
                let n = self.levels.ok_or("VTX power level count is unknown")?;
                if v.len() != n + 1
                    || !v[1..]
                        .iter()
                        .all(|v| !v.is_empty() && v.len() <= 3 && v.is_ascii())
                {
                    return Err("Invalid VTX power labels".into());
                }
                self.labels = Some(v[1..].iter().map(|v| v.to_uppercase()).collect());
            }
            Some("band") => {
                let bands = self.bands.ok_or("VTX band count is unknown")?;
                let channels = self.channels.ok_or("VTX channel count is unknown")?;
                let index = count(v.get(1))?;
                if index == 0
                    || index > bands
                    || v.len() < 4
                    || v[2].is_empty()
                    || v[2].len() > 8
                    || !v[2].is_ascii()
                    || v[3].len() != 1
                    || !v[3].is_ascii()
                {
                    return Err("Invalid VTX band identity".into());
                }
                let flag = v
                    .get(4)
                    .is_some_and(|f| matches!(f.as_str(), "FACTORY" | "CUSTOM"));
                let start = if flag { 5 } else { 4 };
                if v.len() != start + channels
                    || !v[start..].iter().all(|v| v.parse::<u16>().is_ok())
                {
                    return Err("VTX frequencies do not match the channel count".into());
                }
                let mut normalized = vec![
                    v[2].to_uppercase(),
                    v[3].to_uppercase(),
                    if flag { v[4].clone() } else { "CUSTOM".into() },
                ];
                normalized.extend_from_slice(&v[start..]);
                self.rows.insert(index, normalized);
            }
            _ => return Err("Unsupported or malformed VTX table command".into()),
        }
        Ok(())
    }
    pub fn from_config(d: &ConfigDocument) -> Result<Self, String> {
        let mut table = Self::default();
        for l in &d.syntax {
            match &l.command {
                Command::Defaults => table = Self::default(),
                Command::Collection { name, operands } if name == "vtxtable" => {
                    table.apply(operands)?
                }
                _ => {}
            }
        }
        Ok(table)
    }
    pub fn lines(&self) -> Result<Vec<String>, String> {
        let (bands, channels, levels) = (
            self.bands.ok_or("VTX band count is unknown")?,
            self.channels.ok_or("VTX channel count is unknown")?,
            self.levels.ok_or("VTX power level count is unknown")?,
        );
        if bands == 0 || channels == 0 || levels == 0 {
            return Err("Empty VTX tables cannot be exported as a configured VTX".into());
        }
        let mut lines = vec![
            format!("vtxtable bands {bands}"),
            format!("vtxtable channels {channels}"),
        ];
        for i in 1..=bands {
            let row = self
                .rows
                .get(&i)
                .ok_or_else(|| format!("VTX band {i} is unknown"))?;
            lines.push(format!("vtxtable band {i} {}", row.join(" ")));
        }
        lines.push(format!("vtxtable powerlevels {levels}"));
        lines.push(format!(
            "vtxtable powervalues {}",
            self.values
                .as_ref()
                .ok_or("VTX power values are unknown")?
                .join(" ")
        ));
        lines.push(format!(
            "vtxtable powerlabels {}",
            self.labels
                .as_ref()
                .ok_or("VTX power labels are unknown")?
                .join(" ")
        ));
        Ok(lines)
    }
    pub fn validate_selection(&self, d: &ConfigDocument) -> Result<(), String> {
        for (key, max) in [
            ("vtx_band", self.bands),
            ("vtx_channel", self.channels),
            ("vtx_power", self.levels),
        ] {
            let n = d
                .number(&crate::Scope::Global, key)
                .ok_or_else(|| format!("{key} is required for VTX export"))?;
            if n < 1.0 || n > max.unwrap_or(0) as f64 {
                return Err(format!("{key} is outside the explicit VTX table; direct-frequency export is not supported"));
            }
        }
        Ok(())
    }
}
