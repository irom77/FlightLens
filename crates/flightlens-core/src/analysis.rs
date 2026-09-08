// Rate equations derived from Betaflight fc/rc.c (GPL-3.0-or-later).
use crate::{compatibility, ConfigDocument, Scope};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Curve {
    pub name: String,
    pub points: Vec<Point>,
    pub reason: Option<String>,
    /// Inputs this curve read back from the firmware's reset table because the
    /// source omits them, in the order the rate law consumes them. Empty when
    /// every input is declared.
    pub derived_inputs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OsdElement {
    pub key: String,
    pub packed: u32,
    pub x: u32,
    pub y: u32,
    pub visible_profiles: u32,
    pub display_type: u32,
    pub line: u32,
    pub preview: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuleEvaluation {
    pub id: String,
    pub status: String,
    pub severity: String,
    pub explanation: String,
    pub lines: Vec<u32>,
    pub reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub rates: Vec<Curve>,
    pub osd: Vec<OsdElement>,
    pub audits: Vec<RuleEvaluation>,
}
pub fn rate_value(
    kind: &str,
    x: f64,
    rc: f64,
    super_rate: f64,
    expo: f64,
    quick_expo: bool,
) -> Result<f64, String> {
    if ![x, rc, super_rate, expo].iter().all(|n| n.is_finite())
        || !(0.0..=255.0).contains(&rc)
        || !(0.0..=255.0).contains(&super_rate)
        || !(0.0..=100.0).contains(&expo)
    {
        return Err("Invalid rate inputs".into());
    }
    let x = x.clamp(-1.0, 1.0);
    let a = x.abs();
    let e = expo / 100.0;
    let y = match kind {
        "ACTUAL" => {
            let c = rc * 10.0;
            c * x + (super_rate * 10.0 - c).max(0.0) * a * (e * x.powi(5) + (1.0 - e) * x)
        }
        "BETAFLIGHT" => {
            let mut r = rc / 100.0;
            if r > 2.0 {
                r += 14.54 * (r - 2.0);
            }
            200.0 * r * (x * a.powi(3) * e + x * (1.0 - e))
                / (1.0 - a * super_rate / 100.0).clamp(0.01, 1.0)
        }
        "KISS" => (2000.0 * (x.powi(3) * e + x * (1.0 - e)) * rc
            / 1000.0
            / (1.0 - a * super_rate / 100.0).clamp(0.01, 1.0))
        .clamp(-1998.0, 1998.0),
        "QUICK" => {
            if rc == 0.0 {
                return Err("QuickRates requires a positive RC rate".into());
            }
            let r = rc * 2.0;
            let max = (super_rate * 10.0).max(r);
            let s = 1.0 - r / max;
            let (curve, factor) = if quick_expo {
                (x.powi(3) * e + x * (1.0 - e), a)
            } else {
                (x, a.powi(3) * e + a * (1.0 - e))
            };
            (curve * r / (1.0 - factor * s).clamp(0.01, 1.0)).clamp(-1998.0, 1998.0)
        }
        _ => return Err("Rate model is not supported".into()),
    };
    Ok(y)
}
pub fn rates(d: &ConfigDocument, profile: u8) -> Vec<Curve> {
    let scope = Scope::Rate(profile);
    ["roll", "pitch", "yaw"]
        .into_iter()
        .map(|axis| {
            // Named before the curve is computed so the provenance is reported
            // even for an axis that ends up suppressed for a different input.
            let kind = d.text_or_default(&scope, "rates_type");
            let derived_inputs = ["rates_type".to_string()]
                .into_iter()
                .chain(["rc_rate", "srate", "expo"].map(|s| format!("{axis}_{s}")))
                .chain((kind.as_deref() == Some("QUICK")).then(|| "quickrates_rc_expo".to_string()))
                .filter(|key| d.derived_value(&scope, key).is_some())
                .collect();
            let compute = || -> Result<Vec<Point>, String> {
                let kind = kind.clone().ok_or("Rate type is unknown")?;
                let number = |suffix: &str| {
                    d.number_or_default(&scope, &format!("{axis}_{suffix}"))
                        .ok_or_else(|| format!("{axis}_{suffix} is unknown or invalid"))
                };
                let rc = number("rc_rate")?;
                let sr = number("srate")?;
                let e = number("expo")?;
                // 4.2 QuickRates has fixed expo behavior and no CLI toggle.
                let quick = if kind == "QUICK"
                    && compatibility::pack(d.firmware.version.as_deref())
                        .is_some_and(|p| p.parameters.contains_key("quickrates_rc_expo"))
                {
                    match d.text_or_default(&scope, "quickrates_rc_expo").as_deref() {
                        Some("ON") => true,
                        Some("OFF") => false,
                        _ => return Err("quickrates_rc_expo is unknown".into()),
                    }
                } else {
                    false
                };
                (0..=200)
                    .map(|i| {
                        let x = i as f64 / 100.0 - 1.0;
                        Ok(Point {
                            x,
                            y: rate_value(&kind, x, rc, sr, e, quick)?,
                        })
                    })
                    .collect()
            };
            match compute() {
                Ok(points) => Curve {
                    name: axis.into(),
                    points,
                    reason: None,
                    derived_inputs,
                },
                Err(reason) => Curve {
                    name: axis.into(),
                    points: Vec::new(),
                    reason: Some(reason),
                    derived_inputs,
                },
            }
        })
        .collect()
}
pub fn decode_osd(packed: u32) -> (u32, u32, u32, u32) {
    (
        (packed & 0x1f) | ((packed & 0x400) >> 5),
        (packed >> 5) & 0x1f,
        (packed >> 11) & 7,
        (packed >> 14) & 3,
    )
}
pub fn inspect(d: &ConfigDocument, profile: u8) -> Inspection {
    let osd = d
        .parameters
        .values()
        .filter(|p| {
            p.scope == Scope::Global
                && p.key.starts_with("osd_")
                && p.key.ends_with("_pos")
                && p.valid
                && p.supported
        })
        .filter_map(|p| {
            let packed = p.value.number()?;
            if !(0.0..=65535.0).contains(&packed) {
                return None;
            }
            let packed = packed as u32;
            let (x, y, visible_profiles, display_type) = decode_osd(packed);
            Some(OsdElement {
                key: p.key.clone(),
                packed,
                x,
                y,
                visible_profiles,
                display_type,
                line: p.line,
                preview: match p.key.as_str() {
                    "osd_vbat_pos" => "16.8V".into(),
                    "osd_rssi_pos" => "99%".into(),
                    "osd_craft_name_pos" => d
                        .craft_name
                        .clone()
                        .unwrap_or("CRAFT".into())
                        .to_uppercase(),
                    "osd_altitude_pos" => "10M".into(),
                    "osd_current_pos" => "12.3A".into(),
                    "osd_mah_drawn_pos" => "123MAH".into(),
                    "osd_flymode_pos" => "ACRO".into(),
                    "osd_gps_sats_pos" => "12".into(),
                    _ => "+".into(),
                },
            })
        })
        .collect();
    Inspection {
        rates: rates(d, profile),
        osd,
        audits: audit(d),
    }
}
/// Discrete static lowpass response. Caller supplies the actual sample rate; never inferred.
pub fn filter_response(kind: &str, cutoff: f64, sample_rate: f64) -> Result<Vec<Point>, String> {
    if !cutoff.is_finite()
        || !sample_rate.is_finite()
        || cutoff <= 0.0
        || sample_rate <= 0.0
        || cutoff >= sample_rate / 2.0
    {
        return Err("Cutoff must be positive and below the known Nyquist frequency".into());
    }
    let stages = match kind {
        "PT1" => 1,
        "PT2" => 2,
        "PT3" => 3,
        "BIQUAD" => 0,
        _ => return Err("Unsupported filter type".into()),
    };
    let correction = match stages {
        2 => 1.553774,
        3 => 1.961459,
        _ => 1.0,
    };
    let omega = 2.0 * std::f64::consts::PI * cutoff / sample_rate;
    let k = omega * correction / (1.0 + omega * correction);
    let alpha = omega.sin() / 2.0_f64.sqrt();
    let a0 = 1.0 + alpha;
    let b0 = (1.0 - omega.cos()) / 2.0 / a0;
    let b1 = 2.0 * b0;
    let b2 = b0;
    let a1 = -2.0 * omega.cos() / a0;
    let a2 = (1.0 - alpha) / a0;
    Ok((0..=256)
        .map(|i| {
            let f = sample_rate / 2.0 * i as f64 / 256.0;
            let w = 2.0 * std::f64::consts::PI * f / sample_rate;
            let mag = if stages > 0 {
                k / (1.0 + (1.0 - k).powi(2) - 2.0 * (1.0 - k) * w.cos()).sqrt()
            } else {
                let norm = |c0: f64, c1: f64, c2: f64| {
                    ((c0 + c1 * w.cos() + c2 * (2.0 * w).cos()).powi(2)
                        + (c1 * w.sin() + c2 * (2.0 * w).sin()).powi(2))
                    .sqrt()
                };
                norm(b0, b1, b2) / norm(1.0, a1, a2)
            };
            Point {
                x: f,
                y: 20.0 * mag.max(1e-12).log10() * if stages > 0 { stages as f64 } else { 1.0 },
            }
        })
        .collect())
}
fn audit(d: &ConfigDocument) -> Vec<RuleEvaluation> {
    let source = "https://github.com/betaflight/betaflight/blob/";
    let version = d.firmware.version.as_deref().unwrap_or("4.5.0");
    let mut rules = Vec::new();
    let mut add = |id: &str,
                   status: &str,
                   severity: &str,
                   explanation: String,
                   lines: Vec<u32>,
                   file: &str| {
        rules.push(RuleEvaluation {
            id: id.into(),
            status: if d.firmware.pack_id.is_none() {
                "not_applicable".into()
            } else {
                status.into()
            },
            severity: severity.into(),
            explanation,
            lines,
            reference: format!("{source}{version}/src/main/{file}"),
        })
    };
    let poles = d.number(&Scope::Global, "motor_poles");
    let raw_poles = d.parameters.get("global:motor_poles");
    let invalid_poles =
        raw_poles.is_some_and(|p| !p.valid) || poles.is_some_and(|n| n < 4.0 || n % 2.0 != 0.0);
    add("motor-poles",if invalid_poles{"finding"}else if poles.is_some(){"pass"}else{"insufficient_data"},"warning",if invalid_poles{"Motor pole count is invalid or odd."}else{"Configured pole count check only; physical motor pole count cannot be established from a backup."}.into(),raw_poles.map(|p|vec![p.line]).unwrap_or_default(),"cli/settings.c");
    let bidir = d.text(&Scope::Global, "dshot_bidir");
    let protocol = d.text(&Scope::Global, "motor_pwm_protocol");
    let conflict = bidir.as_deref() == Some("ON")
        && protocol.as_ref().is_some_and(|p| !p.starts_with("DSHOT"));
    add("bidirectional-dshot",if bidir.as_deref()==Some("OFF"){"not_applicable"}else if conflict{"finding"}else if bidir.is_some() && protocol.is_some(){"pass"}else{"insufficient_data"},"warning","Bidirectional DShot requires a DShot motor protocol; ESC support and wiring are not established here.".into(),["dshot_bidir","motor_pwm_protocol"].iter().filter_map(|k|d.parameter(&Scope::Global,k).map(|p|p.line)).collect(),"config/config.c");
    let overlapping: Vec<_> = d
        .modes
        .iter()
        .filter(|a| {
            // An unassignable channel cannot activate, so two such rows are not
            // a real overlap however their ranges are declared.
            a.start < a.end
                && a.channel_assigned
                && d.modes.iter().any(|b| {
                    a.index != b.index
                        && a.channel == b.channel
                        && a.mode_id == b.mode_id
                        && a.start < b.end
                        && b.start < a.end
                })
        })
        .map(|m| m.line)
        .collect();
    add("mode-overlap",if !overlapping.is_empty(){"finding"}else if d.modes.is_empty(){"insufficient_data"}else{"pass"},"info","Overlapping ranges for the same mode and AUX channel may be intentional; inspect logic and linked modes.".into(),overlapping,"fc/rc_modes.c");
    let arm: Vec<_> = d
        .modes
        .iter()
        .filter(|m| m.mode_id == 0 && m.start < m.end)
        .map(|m| m.line)
        .collect();
    add("arming-path",if arm.is_empty(){"insufficient_data"}else{"pass"},"info","Checks for a declared ARM range. Missing ranges do not establish that arming is impossible; stick arming and other prerequisites are not reconstructed.".into(),arm,"fc/rc_controls.c");
    add("uart-sharing","insufficient_data","info","Port masks are decoded. Build-specific port-sharing validation is not yet certified; multiple function bits alone are not a collision.".into(),Vec::new(),"io/serial.c");
    add("failsafe","insufficient_data","info","Failsafe procedure can be inspected in Raw. Receiver behavior, rescue prerequisites and in-flight behavior require additional evidence.".into(),Vec::new(),"flight/failsafe.c");
    add("dterm-filters","insufficient_data","info","No universal safe PID threshold is applied. Filter configuration and PID gains alone cannot establish aircraft response.".into(),Vec::new(),"flight/pid.c");
    add("deadband","insufficient_data","info","No heuristic has been certified. Asymmetric deadband settings alone are not evidence of danger.".into(),Vec::new(),"fc/rc.c");
    rules
}
