use flightlens_core::{analysis::*, export::*, parser::parse_line, source::*, *};
fn config(text: &str) -> ConfigDocument {
    let Artifact::Config(d) = analyze(text, "test", "test-source").unwrap() else {
        panic!("expected config")
    };
    *d
}
fn header() -> &'static str {
    "# Betaflight / STM32F405 4.5.0"
}
fn with(body: &str) -> ConfigDocument {
    config(&format!("{}\n{body}", header()))
}
fn fixture(version: &str) -> ConfigDocument {
    config(
        &std::fs::read_to_string(format!("../../fixtures/configs/betaflight-{version}.dump"))
            .unwrap(),
    )
}
fn request(d: &ConfigDocument, groups: &[&str]) -> ExportRequest {
    ExportRequest {
        groups: groups.iter().map(|g| g.to_string()).collect(),
        pid_profile: 0,
        rate_profile: 0,
        destination_header: d.firmware.header.clone().unwrap(),
        include_save: false,
    }
}
#[test]
fn lossless_bom_crlf_and_spans() {
    let s = format!(
        "\u{feff}{}\r\n\r\nset craft_name = a = b\r\nresource MOTOR 1 A00",
        header()
    );
    let d = config(&s);
    assert_eq!(
        d.syntax.iter().map(|l| l.raw.as_str()).collect::<String>(),
        s
    );
    for l in &d.syntax {
        assert_eq!(&s[l.start as usize..l.end as usize], l.raw);
    }
    assert_eq!(
        d.text(&Scope::Global, "craft_name").as_deref(),
        Some("a = b")
    );
    assert!(matches!(
        d.syntax.last().unwrap().command,
        Command::Unsupported
    ));
}
#[test]
fn independent_profiles_and_restore() {
    let d=with("profile 0\nrateprofile 1\nset p_roll = 10\nset roll_rc_rate = 20\nprofile 1\nset p_roll = 40\nset roll_rc_rate = 30\nprofile 0\nrateprofile 0\n");
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), Some(10.));
    assert_eq!(d.number(&Scope::Pid(1), "p_roll"), Some(40.));
    assert_eq!(d.number(&Scope::Rate(1), "roll_rc_rate"), Some(30.));
    assert_eq!(d.selected_pid, Some(0));
}
#[test]
fn no_implicit_profile_or_defaults() {
    let d = with("set p_roll = 40\n");
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), None);
    assert_eq!(d.parameters.len(), 1);
    assert_eq!(d.completeness, "partial");
    assert!(rates(&d, 0).iter().all(|c| c.reason.is_some()));
}
#[test]
fn resets_in_source_order() {
    let d=with("defaults nosave\nset p_roll = 10\nserial 0 64 115200 57600 0 115200\nfeature OSD\ndefaults nosave\nset i_roll = 42\n");
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), None);
    assert_eq!(d.number(&Scope::Pid(0), "i_roll"), Some(42.));
    assert!(d.ports.is_empty());
    assert!(d.features.is_empty());
}
#[test]
fn invalid_known_lines_are_preserved() {
    let text = include_str!("../../../fixtures/malformed/commands.diff");
    let d = config(text);
    assert_eq!(d.syntax.len(), text.lines().count());
    assert!(
        d.diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .count()
            >= 4
    );
    assert!(export(&d, &request(&d, &["rates"])).is_err());
}
#[test]
fn malformed_selector_does_not_leak_scope() {
    let d = with("profile 0\nprofile garbage\nset p_roll = 30\n");
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), None);
    assert_eq!(d.selected_pid, None);
}
#[test]
fn malformed_assignment_does_not_resurrect_previous_value() {
    let d = with("profile 0\nset p_roll = 30\nset p_roll = bad\n");
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), None);
}
#[test]
fn headerless_and_future_versions_are_partial() {
    for s in [
        "set roll_rc_rate = 20\n".into(),
        format!(
            "{}\nprofile 0\nset p_roll = 30\n",
            header().replace("4.5.0", "4.5.1")
        ),
    ] {
        let d = config(&s);
        assert!(d.firmware.pack_id.is_none());
        assert!(rates(&d, 0).iter().all(|c| c.reason.is_some()));
    }
}
#[test]
fn vendor_suffix_versions_use_their_major_minor_schema() {
    let text = format!(
        "{}\nprofile 0\nset p_roll = 55\nrateprofile 0\nset rates_type = BETAFLIGHT\nset roll_rc_rate = 88\nset roll_srate = 70\nset roll_expo = 0\n",
        header().replace("4.5.0", "4.5.3.KAACK_V19")
    );
    let d = config(&text);
    assert_eq!(d.firmware.version.as_deref(), Some("4.5.3.KAACK_V19"));
    assert_eq!(
        d.firmware.pack_id.as_deref(),
        Some("betaflight-4.5.0-schema-1")
    );
    assert_eq!(d.number(&Scope::Pid(0), "p_roll"), Some(55.));
    assert_eq!(d.number(&Scope::Rate(0), "roll_rc_rate"), Some(88.));
    assert!(rates(&d, 0)[0].reason.is_none());
}
#[test]
fn recognized_artifacts_do_not_enter_cli() {
    for text in [
        "H Product:Blackbox flight data recorder by Nicholas Sherlock\n",
        "# INAV / MATEKF405 7.0.0\n",
        "ATC_RAT_RLL_P,0.12\n",
    ] {
        assert!(matches!(
            analyze(text, "test", "source").unwrap(),
            Artifact::Recognized(_)
        ));
    }
}
#[test]
fn import_limits_and_binary_checks() {
    assert!(analyze("", "test", "s").is_err());
    assert!(analyze("abc\0def", "test", "s").is_err());
    assert!(analyze(&"a".repeat(TEXT_LIMIT + 1), "test", "s").is_err());
}
#[test]
fn hash_identity_and_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("配置.dump");
    let text = format!("{}\nset craft_name = Original\n", header());
    std::fs::write(&path, &text).unwrap();
    let mut registry = SourceRegistry::default();
    assert!(registry.path("not-granted").is_err());
    let desc = registry.register(path.clone()).unwrap();
    let Artifact::Config(d) = open_path(&registry.path(&desc.id).unwrap(), &desc.id).unwrap()
    else {
        panic!()
    };
    std::fs::write(&path, "changed").unwrap();
    assert_eq!(d.hash, hash(text.as_bytes()));
    assert_eq!(
        d.syntax.iter().map(|l| l.raw.as_str()).collect::<String>(),
        text
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "changed");
}
#[test]
fn pinned_versions_and_rate_reference() {
    for v in ["4.3.0", "4.4.0", "4.5.0"] {
        let d = fixture(v);
        assert_eq!(d.firmware.version.as_deref(), Some(v));
        assert!(d.firmware.pack_id.is_some());
        assert!(!d.diagnostics.iter().any(|d| d.severity == "error"), "{v}");
        let c = rates(&d, 0);
        assert_eq!(c[0].points[100].y, 0.);
        assert!((c[0].points[150].y - 179.6875).abs() < 1e-9);
        assert_eq!(c[0].points[200].y, 800.);
    }
}
#[test]
fn rate_symmetry_endpoints_and_validation() {
    for model in ["ACTUAL", "BETAFLIGHT", "KISS", "QUICK"] {
        for expo in [0., 50., 100.] {
            for quick in [false, true] {
                for x in [0., 0.1, 0.5, 1.] {
                    let a = rate_value(model, x, 20., 80., expo, quick).unwrap();
                    let b = rate_value(model, -x, 20., 80., expo, quick).unwrap();
                    assert!((a + b).abs() < 1e-8);
                }
            }
        }
    }
    assert!(rate_value("ACTUAL", f64::NAN, 20., 80., 50., false).is_err());
    assert!(rate_value("QUICK", 1., 0., 80., 50., false).is_err());
    assert_eq!(
        rate_value("ACTUAL", 2., 20., 80., 50., false).unwrap(),
        800.
    );
    assert_eq!(
        rate_value("ACTUAL", 1., 80., 20., 50., false).unwrap(),
        800.
    );
}
#[test]
fn osd_all_packed_values_round_trip() {
    for packed in 0..=65535 {
        let (x, y, profiles, ty) = decode_osd(packed);
        let encoded = (x & 31) | ((x & 32) << 5) | (y << 5) | (profiles << 11) | (ty << 14);
        assert_eq!(encoded, packed);
    }
    assert_eq!(decode_osd(0x400 | 9 | 0x800).0, 41);
}
#[test]
fn filter_dc_and_nyquist() {
    for kind in ["PT1", "PT2", "PT3", "BIQUAD"] {
        let p = filter_response(kind, 100., 1000.).unwrap();
        assert!(p[0].y.abs() < 1e-8);
        assert!(p[256].y < 0.);
        assert!(p.iter().all(|p| p.y.is_finite()));
    }
    assert!(filter_response("PT1", 500., 1000.).is_err());
    assert!(filter_response("PT1", 100., f64::NAN).is_err());
}
#[test]
fn serial_baud_and_aux_zero_based() {
    let d = with("serial 20 1 115200 57600 0 250000\naux 0 0 0 1700 2100 0 0\n");
    assert_eq!(d.ports[0].baud, [115200, 57600, 0, 250000]);
    assert_eq!(d.ports[0].name, "USB VCP");
    assert_eq!(d.modes[0].channel, 0);
    assert_eq!(d.modes[0].name, "ARM");
    assert!(parse_line("aux 0 0 0 1701 2100").is_err());
}
#[test]
fn export_dependency_closure_and_round_trip() {
    let d = fixture("4.5.0");
    let r = request(&d, &["rates", "modes", "serial", "osd"]);
    let snippet = export(&d, &r).unwrap();
    assert!(!snippet
        .text
        .lines()
        .any(|l| l == "save" || l.starts_with("defaults") || l.starts_with("resource")));
    assert!(snippet.text.contains("set rates_type = ACTUAL"));
    assert!(snippet.text.contains("rateprofile 0"));
    let parsed = config(&snippet.text);
    assert_eq!(
        d.number(&Scope::Rate(0), "roll_expo"),
        parsed.number(&Scope::Rate(0), "roll_expo")
    );
    assert_eq!(snippet.text, export(&d, &r).unwrap().text);
}
#[test]
fn export_blocks_missing_rates_cross_target_and_invalid_values() {
    let d = with("rateprofile 0\nset rates_type = ACTUAL\nset roll_rc_rate = 20\n");
    assert!(export(&d, &request(&d, &["rates"])).is_err());
    let d = fixture("4.5.0");
    let mut r = request(&d, &["rates"]);
    r.destination_header += " different target";
    assert!(export(&d, &r).is_err());
    let mut r = request(&d, &["rates"]);
    r.include_save = true;
    assert!(export(&d, &r).unwrap().text.ends_with("save\n"));
}
#[test]
fn audit_positive_negative_insufficient() {
    let missing = with("");
    let finding = with("set motor_poles = 13\n");
    let pass = with("set motor_poles = 14\n");
    for (d, status) in [
        (missing, "insufficient_data"),
        (finding, "finding"),
        (pass, "pass"),
    ] {
        assert_eq!(
            inspect(&d, 0)
                .audits
                .iter()
                .find(|r| r.id == "motor-poles")
                .unwrap()
                .status,
            status
        );
    }
}
#[test]
fn serialization_contract_is_camel_case_and_tagged() {
    let d = fixture("4.5.0");
    let json = serde_json::to_value(Artifact::Config(Box::new(d))).unwrap();
    assert_eq!(json["kind"], "config");
    assert!(json["document"]["sourceId"].is_string());
    assert_eq!(
        json["document"]["parameters"]["rate:0:roll_expo"]["scope"]["kind"],
        "rate"
    );
}

#[test]
fn rates_match_pinned_upstream_c_functions() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/rate-vectors.json")).unwrap();
    for v in vectors.as_array().unwrap() {
        let actual = rate_value(
            v["model"].as_str().unwrap(),
            v["x"].as_f64().unwrap(),
            v["rc"].as_f64().unwrap(),
            v["superRate"].as_f64().unwrap(),
            v["expo"].as_f64().unwrap(),
            v["quickExpo"].as_bool().unwrap(),
        )
        .unwrap();
        let expected = v["y"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 0.02,
            "{} {}: {} != {}",
            v["version"],
            v["model"],
            actual,
            expected
        );
    }
}

#[test]
fn vtx_export_requires_a_complete_explicit_table() {
    let d=with("vtxtable bands 1\nvtxtable channels 2\nvtxtable band 1 TEST T CUSTOM 5800 5820\nvtxtable powerlevels 2\nvtxtable powervalues 14 20\nvtxtable powerlabels 25 100\nset vtx_band = 1\nset vtx_channel = 2\nset vtx_power = 1\n");
    let snippet = export(&d, &request(&d, &["vtx"])).unwrap();
    assert!(snippet
        .text
        .contains("vtxtable band 1 TEST T CUSTOM 5800 5820"));
    let partial = with("set vtx_band = 1\nset vtx_channel = 1\nset vtx_power = 1\n");
    assert!(export(&partial, &request(&partial, &["vtx"])).is_err());
    let wrong = with("vtxtable bands 1\nvtxtable channels 2\nvtxtable band 1 TEST T CUSTOM 5800\n");
    assert!(flightlens_core::vtx::Table::from_config(&wrong).is_err());
}
#[test]
fn all_export_groups_reparse_for_each_certified_tag() {
    for v in ["4.3.0", "4.4.0", "4.5.0"] {
        let d = fixture(v);
        for group in ["rates", "pids_filters", "modes", "serial", "osd"] {
            assert!(
                export(&d, &request(&d, &[group])).is_ok(),
                "{v} {group}: {:?}",
                export(&d, &request(&d, &[group]))
            );
        }
    }
}
