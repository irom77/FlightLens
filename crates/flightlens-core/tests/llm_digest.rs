use flightlens_core::{llm::*, *};
fn document(body: &str) -> ConfigDocument {
    let Artifact::Config(d) = analyze(
        &format!("# Betaflight / STM32F405 4.5.0\n{body}"),
        "PRIVATE-FILENAME",
        "PRIVATE-PATH",
    )
    .unwrap() else {
        panic!("config");
    };
    *d
}
fn row() -> DiffRowInput {
    DiffRowInput {
        section: "parameters".into(),
        key: "roll_p".into(),
        scope: "PID 0".into(),
        status: "not_comparable".into(),
        values: vec![Some("42".into()), None],
        reason: Some("Different firmware".into()),
    }
}
#[test]
fn inspector_excludes_source_identity_unknown_keys_and_diagnostic_messages() {
    let mut d = document("# name: PRIVATE-NAME\nset pilot_name = PRIVATE-PILOT\nset vendor_secret = PRIVATE-VENDOR\nset motor_poles = PRIVATE-INVALID\n");
    d.diagnostics.push(Diagnostic {
        line: None,
        severity: "PRIVATE-SEVERITY".into(),
        message: "PRIVATE-MESSAGE".into(),
    });
    let digest = InspectorDigest::build(&d, 0, 0);
    let json = serde_json::to_string(&digest).unwrap();
    assert!(!json.contains("PRIVATE"));
    assert!(!json.contains("vendor_secret"));
    assert!(!json.contains(&d.hash));
    assert_eq!(digest.label, "Backup A");
    let motor = digest
        .parameters
        .iter()
        .find(|p| p.semantic_key == "motor_poles")
        .unwrap();
    assert_eq!(motor.provenance, Provenance::Unknown);
    assert!(motor.value.is_none());
    assert_eq!(digest.diagnostics["other"], 1);
    assert_eq!(digest.audits.len(), analysis::inspect(&d, 0).audits.len());
    assert!(digest
        .audits
        .iter()
        .any(|a| a.status == "insufficient_data"));
}
#[test]
fn profiles_and_recovered_values_remain_distinct_from_unknown() {
    let d = document("defaults nosave\nprofile 0\nset p_pitch = 40\nprofile 1\nset p_pitch = 70\nrateprofile 0\nset roll_rc_rate = 99\nrateprofile 1\nset roll_rc_rate = 111\n");
    let digest = InspectorDigest::build(&d, 1, 0);
    let parameter = |key: &str| {
        digest
            .parameters
            .iter()
            .find(|p| p.semantic_key == key)
            .unwrap()
    };
    assert_eq!(parameter("p_pitch").value.as_deref(), Some("40"));
    assert_eq!(parameter("roll_rc_rate").value.as_deref(), Some("111"));
    assert_eq!(parameter("roll_rc_rate").provenance, Provenance::Declared);
    assert_eq!(parameter("roll_expo").provenance, Provenance::Derived);
    assert_eq!(parameter("motor_poles").provenance, Provenance::Unknown);
    assert!(parameter("motor_poles").value.is_none());
}
#[test]
fn curves_are_bounded_and_unavailable_reasons_and_dump_evidence_survive() {
    let d = document("# dump all\ndefaults nosave\nrateprofile 0\n");
    let digest = InspectorDigest::build(&d, 0, 0);
    assert!(digest.firmware.has_dump_all);
    let inspection = analysis::inspect(&d, 0);
    for (short, full) in digest.rates.iter().zip(&inspection.rates) {
        assert!(short.available);
        assert_eq!(short.samples.len(), 5);
        assert_eq!(short.samples[0].x, full.points[0].x);
        assert_eq!(short.samples[2].y, full.points[full.points.len() / 2].y);
        assert_eq!(short.samples[4].y, full.points.last().unwrap().y);
    }
    assert!(digest.throttle.samples.len() <= 5);
    let absent = InspectorDigest::build(&document(""), 0, 0);
    assert!(!absent.firmware.has_dump_all);
    assert!(absent
        .rates
        .iter()
        .all(|c| !c.available && c.samples.is_empty() && c.reason.is_some()));
    assert!(!absent.throttle.available);
    assert!(absent.throttle.reason.is_some());
}
#[test]
fn diff_rejects_invalid_inputs_including_after_row_cap() {
    for (field, value, expected) in [
        ("key", "pilot_name", DigestError::SensitiveKey),
        ("section", "raw", DigestError::InvalidSection),
        ("status", "unchanged", DigestError::InvalidStatus),
    ] {
        let mut rows = vec![row(); MAX_DIFF_ROWS + 1];
        let last = rows.last_mut().unwrap();
        match field {
            "key" => last.key = value.into(),
            "section" => last.section = value.into(),
            _ => last.status = value.into(),
        }
        assert_eq!(DiffDigest::build(2, None, &rows).err(), Some(expected));
    }
    assert_eq!(
        DiffDigest::build(1, None, &[]).err(),
        Some(DigestError::InvalidSlots)
    );
    assert_eq!(
        DiffDigest::build(4, None, &[]).err(),
        Some(DigestError::InvalidSlots)
    );
    assert_eq!(
        DiffDigest::build(2, Some(2), &[]).err(),
        Some(DigestError::InvalidBaseline)
    );
    assert_eq!(
        DiffDigest::build(3, None, &[row()]).err(),
        Some(DigestError::InvalidValues)
    );
}
#[test]
fn diff_preserves_status_unknowns_order_and_sanitized_labels() {
    let digest = DiffDigest::build(2, Some(1), &[row()]).unwrap();
    assert_eq!(digest.labels, ["Backup A", "Backup B"]);
    assert_eq!(digest.baseline, Some(1));
    assert_eq!(digest.rows[0].status, "not_comparable");
    assert_eq!(digest.rows[0].values, [Some("42".into()), None]);
    assert!(digest.truncated.is_none());
    let mut r = row();
    r.values.push(Some("50".into()));
    assert_eq!(
        DiffDigest::build(3, None, &[r]).unwrap().labels[2],
        "Backup C"
    );
}
#[test]
fn caps_count_characters_and_report_all_omitted_sections() {
    let mut r = row();
    r.values[0] = Some("界".repeat(MAX_FIELD_CHARS + 1));
    r.reason = r.values[0].clone();
    r.scope = r.reason.clone().unwrap();
    r.key = r.scope.clone();
    let mut rows = vec![r; MAX_DIFF_ROWS + 2];
    rows[MAX_DIFF_ROWS].section = "ports".into();
    let digest = DiffDigest::build(2, None, &rows).unwrap();
    assert_eq!(digest.rows.len(), MAX_DIFF_ROWS);
    assert_eq!(
        digest.truncated.unwrap(),
        Truncation {
            omitted: 2,
            sections: vec!["parameters".into(), "ports".into()]
        }
    );
    let r = &digest.rows[0];
    for value in [
        &r.key,
        &r.scope,
        r.reason.as_ref().unwrap(),
        r.values[0].as_ref().unwrap(),
    ] {
        assert_eq!(value.chars().count(), MAX_FIELD_CHARS);
        assert!(value.ends_with("[truncated]"));
    }
    let mut exact = row();
    exact.values[0] = Some("a".repeat(MAX_FIELD_CHARS));
    assert_eq!(
        DiffDigest::build(2, None, &[exact]).unwrap().rows[0].values[0]
            .as_ref()
            .unwrap()
            .len(),
        MAX_FIELD_CHARS
    );
}
#[test]
fn prompts_rebuild_identically_and_round_trip_with_explicit_nulls() {
    let d = document("");
    let digest = InspectorDigest::build(&d, 0, 0);
    let json = serde_json::to_string_pretty(&digest).unwrap();
    assert!(json.contains("\"value\": null"));
    assert!(json.contains("\"provenance\": \"unknown\""));
    let restored: InspectorDigest = serde_json::from_str(&json).unwrap();
    let preview = inspector_prompt(&digest);
    assert_eq!(preview, inspector_prompt(&restored));
    assert_eq!(preview, inspector_prompt(&InspectorDigest::build(&d, 0, 0)));
    assert!(preview.user.ends_with(&json));
    let diff = DiffDigest::build(2, None, &[row()]).unwrap();
    assert_eq!(
        diff_prompt(&diff),
        diff_prompt(&DiffDigest::build(2, None, &[row()]).unwrap())
    );
    for requirement in [
        "untrusted data",
        "unknown",
        "not_comparable",
        "partial",
        "dump all",
        "flight behavior",
        "tuning changes",
    ] {
        assert!(preview.system.contains(requirement));
    }
}
