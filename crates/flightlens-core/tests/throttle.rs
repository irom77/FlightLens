use flightlens_core::{analyze, throttle::preview, Artifact, ConfigDocument};

fn config(version: &str, body: &str) -> ConfigDocument {
    let Artifact::Config(d) = analyze(
        &format!("# Betaflight / STM32F405 {version}\n{body}"),
        "test",
        "test-source",
    )
    .unwrap() else {
        panic!("expected config")
    };
    *d
}
fn settings(mid: u8, expo: u8, limit: &str, percent: u8) -> String {
    format!("rateprofile 0\nset thr_mid = {mid}\nset thr_expo = {expo}\nset throttle_limit_type = {limit}\nset throttle_limit_percent = {percent}\n")
}
#[test]
fn documented_integer_vectors_and_limit_ordering() {
    for (mid, expo, expected) in [
        (50, 0, [0, 100, 250, 300, 500, 750, 900, 999, 1000]),
        (50, 50, [0, 172, 340, 384, 500, 660, 828, 998, 1000]),
        (50, 100, [0, 244, 430, 468, 500, 570, 756, 997, 1000]),
        (30, 70, [0, 178, 281, 300, 370, 566, 786, 997, 1000]),
        (0, 100, [0, 1, 17, 27, 125, 427, 729, 997, 1000]),
    ] {
        let curve = preview(&config("4.5.0", &settings(mid, expo, "OFF", 50)), 0);
        assert!(curve.reason.is_none(), "{:?}", curve.reason);
        assert_eq!(curve.points.len(), 1001);
        for (input, output) in [0, 100, 250, 300, 500, 750, 900, 999, 1000]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(curve.points[input].x, input as f64 / 10.0);
            assert_eq!(curve.points[input].y, output as f64 / 10.0);
        }
    }
    for (limit, expected) in [("OFF", 66.0), ("SCALE", 33.0), ("CLIP", 50.0)] {
        assert_eq!(
            preview(&config("4.5.0", &settings(50, 50, limit, 50)), 0).points[750].y,
            expected
        );
    }
}
#[test]
fn supported_releases_and_identity_gates() {
    for version in [
        "4.2.0", "4.3.0", "4.4.0", "4.5.0", "4.5.1", "4.5.2", "4.5.3", "4.5.4", "4.5.5",
    ] {
        assert!(
            preview(&config(version, &settings(50, 50, "OFF", 100)), 0)
                .reason
                .is_none(),
            "{version}"
        );
    }
    for version in [
        "4.2.1",
        "4.5.6",
        "4.5.3.KAACK_V18",
        "4.5.3-RC1",
        "2025.12.1",
        "2025.12.5",
        "unknown",
    ] {
        let curve = preview(&config(version, &settings(50, 50, "OFF", 100)), 0);
        assert!(curve.reason.is_some(), "{version}");
        assert!(curve.points.is_empty());
    }
    let mut d = config("4.5.0", &settings(50, 50, "OFF", 100));
    d.firmware.family = "inav".into();
    assert!(preview(&d, 0).reason.is_some());
    d.firmware.family = "betaflight".into();
    d.firmware.pack_id = Some("wrong-pack".into());
    assert!(preview(&d, 0).reason.is_some());
}
#[test]
fn explicit_valid_profile_inputs_required() {
    let body = settings(50, 50, "OFF", 100);
    for key in [
        "thr_mid",
        "thr_expo",
        "throttle_limit_type",
        "throttle_limit_percent",
    ] {
        let missing = body
            .lines()
            .filter(|line| !line.starts_with(&format!("set {key} =")))
            .collect::<Vec<_>>()
            .join("\n");
        let curve = preview(&config("4.5.0", &missing), 0);
        assert!(curve.reason.unwrap().contains(key));
        assert!(curve.points.is_empty());
        let invalid = format!("{body}set {key} = invalid\n");
        assert!(preview(&config("4.5.0", &invalid), 0)
            .reason
            .unwrap()
            .contains(key));
    }
    for (key, value) in [
        ("thr_mid", "101"),
        ("thr_expo", "-1"),
        ("thr_expo", "1.5"),
        ("throttle_limit_percent", "24"),
    ] {
        assert!(
            preview(&config("4.5.0", &format!("{body}set {key} = {value}\n")), 0)
                .reason
                .is_some()
        );
    }
    let d = config(
        "4.5.0",
        &format!(
            "{body}{}",
            settings(30, 70, "OFF", 100).replace("rateprofile 0", "rateprofile 1")
        ),
    );
    assert_eq!(preview(&d, 0).points[500].y, 50.0);
    assert_eq!(preview(&d, 1).points[500].y, 37.0);
    assert!(preview(&d, 2).reason.is_some());
    assert!(preview(&d, 0).derived_inputs.is_empty());
    assert!(preview(&config("4.5.0", &settings(100, 50, "OFF", 100)), 0)
        .reason
        .unwrap()
        .contains("CLI value itself is valid"));
}
#[test]
fn zero_expo_is_linear_and_curves_are_bounded_monotonic() {
    for mid in [0, 1, 30, 50, 99] {
        for expo in [0, 50, 100] {
            let curve = preview(&config("4.5.0", &settings(mid, expo, "OFF", 100)), 0);
            assert_eq!(curve.points.first().unwrap().y, 0.0);
            assert_eq!(curve.points.last().unwrap().y, 100.0);
            assert!(curve.points.windows(2).all(|p| p[0].y <= p[1].y));
            if expo == 0 {
                assert!(curve.points.iter().all(|p| p.x == p.y));
            }
        }
    }
}

#[test]
fn matches_pinned_upstream_c_lookup_and_limits() {
    let releases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/throttle-vectors.json")).unwrap();
    assert_eq!(releases.as_array().unwrap().len(), 9);
    for release in releases.as_array().unwrap() {
        let version = release["version"].as_str().unwrap();
        let mut previous = None;
        let mut curve = None;
        for vector in release["vectors"].as_array().unwrap() {
            let values = vector.as_array().unwrap();
            let mid = values[0].as_u64().unwrap() as u8;
            let expo = values[1].as_u64().unwrap() as u8;
            let mode = values[2].as_u64().unwrap() as usize;
            let percent = values[3].as_u64().unwrap() as u8;
            let input = values[4].as_u64().unwrap() as usize;
            let case = (mid, expo, mode, percent);
            if previous != Some(case) {
                curve = Some(preview(
                    &config(
                        version,
                        &settings(mid, expo, ["OFF", "SCALE", "CLIP"][mode], percent),
                    ),
                    0,
                ));
                previous = Some(case);
            }
            let point = &curve.as_ref().unwrap().points[input];
            let expected = values[6].as_f64().unwrap();
            assert!(
                (point.y - expected).abs() < 0.00002,
                "{version} {case:?} input={input}: {} != {expected}",
                point.y
            );
            if mode == 0 {
                assert_eq!(point.y, values[5].as_f64().unwrap() / 10.0);
            }
        }
    }
}
