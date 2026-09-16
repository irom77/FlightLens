//! Static Betaflight throttle command, excluding runtime mixer effects.
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
                    | "4.2.1"
                    | "4.2.2"
                    | "4.2.3"
                    | "4.2.4"
                    | "4.2.5"
                    | "4.2.6"
                    | "4.2.7"
                    | "4.2.8"
                    | "4.2.9"
                    | "4.2.10"
                    | "4.2.11"
                    | "4.3.0"
                    | "4.3.1"
                    | "4.3.2"
                    | "4.4.0"
                    | "4.4.1"
                    | "4.4.2"
                    | "4.4.3"
                    | "4.5.0"
                    | "4.5.1"
                    | "4.5.2"
                    | "4.5.3"
                    | "4.5.4"
                    | "4.5.5"
                    | "4.5.3.KAACK_V19"
                    | "2025.12.3-alpha.KAACK_V19"
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
    let hover = if version == Some("2025.12.3-alpha.KAACK_V19") {
        Some(integer("thr_hover", 0, 100)?)
    } else {
        None
    };
    if mid == 100 && hover.is_none() {
        return Err("Throttle preview is unavailable for thr_mid=100: the firmware's extra lookup knot has a zero divisor; the CLI value itself is valid".into());
    }
    // Only knots used in the input domain. The firmware's extra knot at 110%
    // has zero interpolation weight at full input and is intentionally omitted.
    let knots: Vec<i32> = if let Some(hover) = hover {
        hover_knots(mid, expo, hover)
    } else {
        (0..=10)
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
            .collect()
    };
    Ok((0..=1000)
        .map(|input| {
            let index = input / 100;
            let command = if hover.is_some() {
                let scaled = input * 11;
                let index = scaled / 1000;
                if index == 11 {
                    knots[11]
                } else {
                    knots[index] + (scaled % 1000) as i32 * (knots[index + 1] - knots[index]) / 1000
                }
            } else if input == 1000 {
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

// Preserve firmware f32 evaluation and lrintf's default ties-to-even rounding.
fn hover_knots(mid: i32, expo: i32, hover: i32) -> Vec<i32> {
    let mid = mid as f32 / 100.0;
    let expo = expo as f32 / 100.0;
    let hover = hover as f32 / 100.0;
    let cp1x = mid * 0.5;
    let cp1y = hover * 0.5 * (1.0 + expo);
    let cp2x = (1.0 + mid) * 0.5;
    let cp2y = 1.0 + ((hover - 1.0) * 0.5 * (1.0 + expo));
    (0..12)
        .map(|i| {
            let x = i as f32 / 11.0;
            let y = if x <= mid {
                quadratic_bezier(x, [0.0, cp1x, mid], [0.0, cp1y, hover])
            } else {
                quadratic_bezier(x, [mid, cp2x, 1.0], [hover, cp2y, 1.0])
            };
            (1000.0 * y + 1000.0).round_ties_even() as i32 - 1000
        })
        .collect()
}

fn quadratic_bezier(x: f32, px: [f32; 3], py: [f32; 3]) -> f32 {
    let a = px[0] - 2.0 * px[1] + px[2];
    let b = 2.0 * px[1] - 2.0 * px[0];
    let c = px[0] - x;
    let mut t = 0.0;
    if a.abs() < 1e-6 {
        if b.abs() > 1e-6 {
            t = -c / b;
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc >= 0.0 {
            let sqrt_d = disc.sqrt();
            let t1 = (-b + sqrt_d) / (2.0 * a);
            let t2 = (-b - sqrt_d) / (2.0 * a);
            t = if (0.0..=1.0).contains(&t1) { t1 } else { t2 };
        }
    }
    t = t.clamp(0.0, 1.0);
    (1.0 - t) * (1.0 - t) * py[0] + 2.0 * (1.0 - t) * t * py[1] + t * t * py[2]
}
