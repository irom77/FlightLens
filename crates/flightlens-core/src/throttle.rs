//! Static legacy Betaflight throttle command, excluding runtime mixer effects.
//! Equations derived from Betaflight fc/rc.c (GPL-3.0-or-later).
//! Support evidence and integer semantics: docs/throttle-curve-preview.md.
use crate::{analysis::Curve, analysis::Point, compatibility, ConfigDocument, Scope};

/// Percentage axes, all 1001 integer normalized input samples. Missing or
/// uncertified inputs produce an empty curve with an explanation, never defaults.
pub fn preview(document: &ConfigDocument, profile: u8) -> Curve {
    let result = calculate(document, profile);
    let (points, reason) = match result {
        Ok(points) => (points, None),
        Err(reason) => (Vec::new(), Some(reason)),
    };
    Curve {
        name: "throttle".into(),
        points,
        reason,
        derived_inputs: Vec::new(),
    }
}

fn calculate(d: &ConfigDocument, profile: u8) -> Result<Vec<Point>, String> {
    let version = d.firmware.version.as_deref();
    if d.firmware.family != "betaflight"
        || !matches!(
            version,
            Some(
                "4.2.0"
                    | "4.3.0"
                    | "4.4.0"
                    | "4.5.0"
                    | "4.5.1"
                    | "4.5.2"
                    | "4.5.3"
                    | "4.5.4"
                    | "4.5.5"
            )
        )
        || !compatibility::pack(version)
            .is_some_and(|pack| d.firmware.pack_id.as_deref() == Some(pack.id.as_str()))
    {
        return Err(
            "Throttle preview is not verified for this firmware release or schema pack".into(),
        );
    }
    let scope = Scope::Rate(profile);
    let integer = |key: &str, min: i32, max: i32| -> Result<i32, String> {
        let value = d
            .number(&scope, key)
            .filter(|value| {
                value.is_finite()
                    && value.fract() == 0.0
                    && *value >= min as f64
                    && *value <= max as f64
            })
            .ok_or_else(|| {
                format!("{key} is missing, invalid, or unsupported in this rate profile")
            })?;
        Ok(value as i32)
    };
    let mid = integer("thr_mid", 0, 100)?;
    let expo = integer("thr_expo", 0, 100)?;
    let percent = integer("throttle_limit_percent", 25, 100)?;
    let limit = d
        .text(&scope, "throttle_limit_type")
        .filter(|value| matches!(value.as_str(), "OFF" | "SCALE" | "CLIP"))
        .ok_or("throttle_limit_type is missing, invalid, or unsupported in this rate profile")?;
    if mid == 100 {
        return Err("Throttle preview is unavailable for thr_mid=100: the firmware's extra lookup knot has a zero divisor; the CLI value itself is valid".into());
    }
    // Only knots used in the input domain. The firmware's extra knot at 110%
    // has zero interpolation weight at full input and is intentionally omitted.
    let knots: Vec<i32> = (0..=10)
        .map(|i| {
            let delta = 10 * i - mid;
            let span = if delta > 0 {
                100 - mid
            } else if delta < 0 {
                mid
            } else {
                1
            };
            10 * mid + delta * (100 - expo + expo * delta * delta / (span * span)) / 10
        })
        .collect();
    Ok((0..=1000)
        .map(|input| {
            let index = input / 100;
            let command = if input == 1000 {
                knots[10]
            } else {
                knots[index] + (input % 100) as i32 * (knots[index + 1] - knots[index]) / 100
            };
            let output = command as f64 / 10.0;
            Point {
                x: input as f64 / 10.0,
                y: match limit.as_str() {
                    "SCALE" => output * percent as f64 / 100.0,
                    "CLIP" => output.min(percent as f64),
                    _ => output,
                },
            }
        })
        .collect())
}
