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
#[test]
fn firmware_versions_preserve_prerelease_and_build_suffixes() {
    for version in [
        "2026.01.0-ALPHA.CUSTOM_A01",
        "2026.01.0",
        "4.5.3-RC1",
        "4.5.3+custom",
        "4.5.3.KAACK_V18",
    ] {
        let d = config(&format!("# Betaflight / STM32F405 {version}\n"));
        assert_eq!(d.firmware.family, "betaflight");
        assert_eq!(d.firmware.version.as_deref(), Some(version));
        assert!(!compatibility::defaults_certified(version));
        if version.starts_with("2026.") {
            assert!(d.firmware.pack_id.is_none());
        }
    }
    for version in ["4.5", "4.5.x", "4.5.3garbage"] {
        let d = config(&format!("# Betaflight / STM32F405 {version}\n"));
        assert!(d.firmware.version.is_none());
    }
}
fn with(body: &str) -> ConfigDocument {
    config(&format!("{}\n{body}", header()))
}
#[test]
fn numeric_feature_names_are_valid_and_last_command_wins() {
    let d = with("feature 3D\nfeature -3D\nfeature OSD\n");
    assert!(!d.diagnostics.iter().any(|x| x.severity == "error"));
    assert_eq!(d.features.get("3D"), Some(&false));
    let d = with("feature -3D\nfeature 3D\n");
    assert_eq!(d.features.get("3D"), Some(&true));
    let base = std::fs::read_to_string("../../fixtures/configs/betaflight-4.5.0.dump").unwrap();
    for command in ["feature 3D", "feature -3D"] {
        let d = config(&format!("{base}{command}\n"));
        for group in ["osd", "serial"] {
            assert!(export(&d, &request(&d, &[group])).is_ok(), "{group}");
        }
    }
    for line in [
        "feature --3D",
        "feature -",
        "feature 3D extra",
        "feature bad!",
    ] {
        assert!(parse_line(line).is_err(), "{line}");
    }
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
fn headerless_and_unsupported_versions_are_partial() {
    for s in [
        "set roll_rc_rate = 20\n".into(),
        format!(
            "{}\nprofile 0\nset p_roll = 30\n",
            header().replace("4.5.0", "4.6.1")
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
    for v in ["4.3.0", "4.4.0", "4.5.0", "2025.12.1"] {
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
    for v in ["4.3.0", "4.4.0", "4.5.0", "2025.12.1"] {
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
#[test]
fn firmware_profile_counts_come_from_the_schema() {
    // Betaflight 4.4 raised the PID profile count from three to four. The
    // fourth profile must be attributed, not rejected as malformed, and the
    // settings that follow it must stay in that scope.
    let d = with("profile 3\nset p_roll = 42\n");
    assert_eq!(d.number(&Scope::Pid(3), "p_roll"), Some(42.0));
    assert_eq!(d.selected_pid, Some(3));
    assert!(d.pid_profiles.contains(&3));
    assert!(!d.diagnostics.iter().any(|x| x.severity == "error"));

    // 4.3 certifies three PID profiles and six rate profiles.
    let older = config(&format!(
        "{}\nrateprofile 5\nset roll_srate = 70\n",
        header().replace("4.5.0", "4.3.0")
    ));
    assert_eq!(older.number(&Scope::Rate(5), "roll_srate"), Some(70.0));
    assert!(!older.diagnostics.iter().any(|x| x.severity == "error"));
}
#[test]
fn out_of_range_profile_warns_without_discarding_settings() {
    // 4.5 certifies four rate profiles. A sixth is reported, but the source is
    // still read as declared rather than silently dropped into unknown scope.
    let d = with("rateprofile 5\nset roll_srate = 70\n");
    assert_eq!(d.number(&Scope::Rate(5), "roll_srate"), Some(70.0));
    assert!(!d.diagnostics.iter().any(|x| x.severity == "error"));
    assert!(d
        .diagnostics
        .iter()
        .any(|x| x.severity == "warning" && x.message.contains("Rate profile 5")));
    // A selector that is not a number remains malformed.
    assert!(parse_line("profile 3").is_ok());
    assert!(parse_line("profile garbage").is_err());
}
#[test]
fn bitset_flags_are_on_off_not_integers() {
    // MODE_BITSET settings print as ON/OFF in the CLI. Typing them as integers
    // made every real dump fail schema validation and blocked export.
    let d = with("set blackbox_disable_acc = OFF\nset telemetry_disabled_pitch = ON\n");
    assert!(!d.diagnostics.iter().any(|x| x.severity == "error"));
    for (key, value) in [
        ("blackbox_disable_acc", "OFF"),
        ("telemetry_disabled_pitch", "ON"),
    ] {
        let p = &d.parameters[&format!("global:{key}")];
        assert!(p.supported, "{key} must resolve to a schema");
        assert!(p.valid, "{key} = {value} must validate");
    }
    // The enum is still closed: a value the firmware never prints is invalid.
    let bad = with("set blackbox_disable_acc = 7\n");
    assert!(bad.diagnostics.iter().any(|x| x.severity == "error"));
}
#[test]
fn unassignable_aux_channel_is_read_and_flagged_not_dropped() {
    // Betaflight prints `auxChannelIndex` as the raw stored byte, so a mode
    // configured without a channel appears as 255 in real 4.3 and 4.4 dumps.
    // The row is the user's configuration and must survive parsing.
    let d = with("aux 2 13 255 1300 1700 0 0\n");
    assert_eq!(d.modes.len(), 1);
    assert_eq!(d.modes[0].channel, 255);
    assert!(!d.modes[0].channel_assigned);
    assert_eq!(d.modes[0].start, 1300);
    assert!(!d.diagnostics.iter().any(|x| x.severity == "error"));
    assert!(d
        .diagnostics
        .iter()
        .any(|x| x.severity == "warning" && x.message.contains("AUX channel 255")));

    // Channel 13 is the last assignable one; 14 is the first that is not.
    assert!(with("aux 0 0 13 1300 1700 0 0\n").modes[0].channel_assigned);
    assert!(!with("aux 0 0 14 1300 1700 0 0\n").modes[0].channel_assigned);
    // A channel that is not a byte at all is still malformed.
    assert!(parse_line("aux 0 0 256 1300 1700 0 0").is_err());
}
#[test]
fn export_refuses_modes_the_firmware_would_discard() {
    // `cliAux` zeroes the whole mode activation condition when it reads back a
    // channel it cannot assign, so exporting the line verbatim would erase the
    // mode rather than restore it.
    let mut d = fixture("4.5.0");
    assert!(export(&d, &request(&d, &["modes"])).is_ok());
    d.modes[0].channel = 255;
    d.modes[0].channel_assigned = false;
    let err = export(&d, &request(&d, &["modes"])).unwrap_err();
    assert!(err.contains("assignable AUX channel"), "{err}");
    // Other groups are unaffected by a mode the request never asked for.
    assert!(export(&d, &request(&d, &["rates"])).is_ok());
}
#[test]
fn blackbox_headers_are_detected_away_from_byte_zero() {
    assert!(is_blackbox(b"H Product:Blackbox flight data recorder"));
    // A byte-order mark, leading junk, or a recovered partial first session
    // pushes the header off the start; such a log is not CLI text.
    let mut shifted = b"\xef\xbb\xbf\n".to_vec();
    shifted.extend_from_slice(b"H Product:Blackbox flight data recorder");
    assert!(is_blackbox(&shifted));
    assert!(!is_blackbox(
        b"# Betaflight / STM32F405 4.5.0\nset roll_srate = 70\n"
    ));
    assert!(!is_blackbox(b"H Product:"));
}
#[test]
fn export_gate_is_scoped_to_the_groups_the_request_asks_for() {
    // A malformed serial line is a real defect, but it cannot reach a rates
    // snippet. Blocking every group on it made one bad line a total outage.
    let mut text = std::fs::read_to_string("../../fixtures/configs/betaflight-4.5.0.dump").unwrap();
    text.push_str("serial 30 nonsense\n");
    let d = config(&text);
    assert!(d.diagnostics.iter().any(|x| x.severity == "error"));
    assert!(export(&d, &request(&d, &["rates"])).is_ok());
    assert!(export(&d, &request(&d, &["modes"])).is_ok());

    let err = export(&d, &request(&d, &["serial"])).unwrap_err();
    assert!(err.contains("serial needs line"), "{err}");
    // The line is named so the message is actionable.
    let line = text.lines().count() as u32;
    assert!(err.contains(&line.to_string()), "{err}");
    // Selecting the affected group alongside a clean one still blocks.
    assert!(export(&d, &request(&d, &["rates", "serial"])).is_err());
}
#[test]
fn export_gate_still_blocks_what_the_snippet_depends_on() {
    let base = std::fs::read_to_string("../../fixtures/configs/betaflight-4.5.0.dump").unwrap();
    // A malformed aux line is the modes group's own dependency.
    let d = config(&format!("{base}aux 1 2 3\n"));
    assert!(export(&d, &request(&d, &["modes"])).is_err());
    assert!(export(&d, &request(&d, &["rates"])).is_ok());
    // Features are emitted for the OSD and serial groups, so a malformed
    // feature line blocks both and nothing else.
    let d = config(&format!("{base}feature OSD extra\n"));
    assert!(export(&d, &request(&d, &["osd"])).is_err());
    assert!(export(&d, &request(&d, &["serial"])).is_err());
    assert!(export(&d, &request(&d, &["rates"])).is_ok());
    // A selector that did not parse leaves the following settings in an
    // indeterminate profile, so its own group can no longer be proven complete.
    let d = config(&format!("{base}rateprofile x\n"));
    assert!(export(&d, &request(&d, &["rates"])).is_err());
    assert!(export(&d, &request(&d, &["modes"])).is_ok());
    // An unrecognised malformed line is unattributable and blocks everything.
    let d = config(&format!("{base}set = 5\n"));
    for group in ["rates", "modes", "serial", "osd"] {
        assert!(export(&d, &request(&d, &[group])).is_err(), "{group}");
    }
}
#[test]
fn invalid_values_block_only_the_group_that_would_carry_them() {
    let base = std::fs::read_to_string("../../fixtures/configs/betaflight-4.5.0.dump").unwrap();
    // An invalid OSD value is attributed to its own parameter, so it blocks the
    // OSD group by name and leaves the rest of the snippet alone.
    let d = config(&format!("{base}set osd_units = FURLONGS\n"));
    let err = export(&d, &request(&d, &["osd"])).unwrap_err();
    assert!(
        err.contains("osd_units") && err.contains("schema rejects"),
        "{err}"
    );
    let line = d.parameters["global:osd_units"].line;
    assert!(err.contains(&format!("line {line}")), "{err}");
    assert!(export(&d, &request(&d, &["rates"])).is_ok());
    assert!(export(&d, &request(&d, &["modes", "serial"])).is_ok());
}

#[test]
fn omitted_rate_values_are_read_back_only_from_a_declared_baseline() {
    // Without `defaults` the file states no baseline, so an omission means
    // nothing and the curve stays suppressed exactly as before.
    let bare = with("rateprofile 0\nset roll_rc_rate = 12\n");
    assert!(bare.derived.is_empty());
    assert!(rates(&bare, 0).iter().all(|c| c.points.is_empty()));

    let d = with("defaults nosave\nrateprofile 0\nset roll_rc_rate = 12\n");
    let scope = Scope::Rate(0);
    // A declared value always wins; only untouched keys are read back.
    assert_eq!(d.number(&scope, "roll_rc_rate"), Some(12.0));
    assert!(d.derived_value(&scope, "roll_rc_rate").is_none());
    let expo = d.derived_value(&scope, "roll_expo").unwrap();
    assert_eq!(expo.raw_value, "0");
    assert_eq!(expo.source_version, "4.5.0");
    assert_eq!(
        d.text_or_default(&scope, "rates_type").as_deref(),
        Some("ACTUAL")
    );
    assert_eq!(d.number_or_default(&scope, "roll_rc_rate"), Some(12.0));
    assert_eq!(d.number_or_default(&scope, "yaw_srate"), Some(67.0));

    let curves = rates(&d, 0);
    assert!(curves.iter().all(|c| c.points.len() == 201));
    let roll = curves.iter().find(|c| c.name == "roll").unwrap();
    assert!(!roll.derived_inputs.contains(&"roll_rc_rate".to_string()));
    assert_eq!(
        roll.derived_inputs,
        ["rates_type", "roll_srate", "roll_expo"]
    );
}

#[test]
fn vendor_builds_and_invalid_values_are_never_filled_in() {
    // A vendor build resolves to its major/minor schema for syntax and bounds,
    // which a custom build cannot change -- but it can change any default, and
    // nothing in the dump says whether it did.
    let vendor =
        config("# Betaflight / STM32F405 4.5.3.KAACK_V19\ndefaults nosave\nrateprofile 0\n");
    assert_eq!(
        vendor.firmware.pack_id.as_deref(),
        Some("betaflight-4.5.0-schema-1")
    );
    assert!(vendor.derived.is_empty());
    assert!(vendor
        .diagnostics
        .iter()
        .any(|x| x.message.contains("not in the verified release list")));

    // A value the schema rejects is declared, so it is not derived either: the
    // curve must stay unavailable rather than silently show the default.
    let d = with("defaults nosave\nrateprofile 0\nset roll_srate = 900\n");
    let scope = Scope::Rate(0);
    assert!(d.derived_value(&scope, "roll_srate").is_none());
    assert_eq!(d.number_or_default(&scope, "roll_srate"), None);
    let curves = rates(&d, 0);
    assert!(curves
        .iter()
        .find(|c| c.name == "roll")
        .unwrap()
        .points
        .is_empty());
    assert!(
        curves
            .iter()
            .find(|c| c.name == "yaw")
            .unwrap()
            .points
            .len()
            == 201
    );
}

#[test]
fn export_never_emits_a_value_the_source_did_not_declare() {
    // Reading a default back is a claim about the file. Writing one to a flight
    // controller would be a line the user never wrote, so the snippet still has
    // to declare all ten rate keys itself.
    let d = with("defaults nosave\nrateprofile 0\nset roll_rc_rate = 12\n");
    assert!(!d.derived.is_empty());
    assert!(rates(&d, 0).iter().all(|c| c.points.len() == 201));
    let error = export(&d, &request(&d, &["rates"])).unwrap_err();
    assert!(error.contains("rates_type"), "{error}");

    let complete = with(&format!(
        "defaults nosave\nrateprofile 0\nset rates_type = ACTUAL\n{}",
        ["roll", "pitch", "yaw"]
            .iter()
            .map(|a| format!("set {a}_rc_rate = 12\nset {a}_srate = 70\nset {a}_expo = 5\n"))
            .collect::<String>()
    ));
    let snippet = export(&complete, &request(&complete, &["rates"])).unwrap();
    for derived in complete.derived.values() {
        assert!(
            !snippet.text.contains(&format!("set {} ", derived.key)),
            "{} was exported but no source line declares it",
            derived.key
        );
    }
    assert!(snippet.text.contains("set roll_srate = 70"));
}

#[test]
fn a_document_that_qualifies_for_no_defaults_says_why() {
    // Silence is the one thing this must not do: the guard that withholds a
    // default has to be as visible as the value it would have supplied.
    let vendor =
        config("# Betaflight / STM32F405 4.5.3.KAACK_V19\ndefaults nosave\nrateprofile 0\n");
    assert!(vendor.derived.is_empty());
    assert!(vendor
        .derived_note
        .as_deref()
        .unwrap()
        .contains("4.5.3.KAACK_V19"));

    // A backup that never resets states no baseline, so an omission carries no
    // information at all -- a different reason, and worth saying so.
    let no_baseline = with("rateprofile 0\nset roll_rc_rate = 12\n");
    assert!(no_baseline.derived.is_empty());
    assert!(no_baseline
        .derived_note
        .as_deref()
        .unwrap()
        .contains("does not reset the configuration"));

    // Nothing to explain when the values are actually there.
    let applied = with("defaults nosave\nrateprofile 0\n");
    assert!(!applied.derived.is_empty());
    assert_eq!(applied.derived_note, None);
}

#[test]
fn betaflight_42_explicit_rates_and_pids_are_inspectable() {
    let d = config(
        &include_str!("../../../fixtures/configs/betaflight-4.5.0.dump")
            .replace("4.5.0", "4.2.11")
            .replace("defaults nosave", "# explicit settings only")
            .replace("dterm_lpf1_static_hz", "dterm_lowpass_hz")
            .replace("dterm_lpf1_type", "dterm_lowpass_type"),
    );
    assert!(d.firmware.pack_id.is_some());
    assert!(rates(&d, 0).iter().all(|c| c.points.len() == 201));
    for axis in ["roll", "pitch", "yaw"] {
        for gain in ["p", "i", "d", "f"] {
            assert!(d
                .number(&Scope::Pid(0), &format!("{gain}_{axis}"))
                .is_some());
        }
    }
    assert_eq!(d.number(&Scope::Pid(0), "d_yaw"), Some(0.0));
    let quick = config(
        &d.syntax
            .iter()
            .map(|line| line.raw.as_str())
            .collect::<String>()
            .replace("ACTUAL", "QUICK"),
    );
    assert!(rates(&quick, 0).iter().all(|c| c.points.len() == 201));
    assert!(export(&quick, &request(&quick, &["rates"])).is_ok());
}

#[test]
fn betaflight_42_defaults_require_verified_release_and_reset() {
    for patch in 0..=11 {
        let d = config(&format!(
            "# Betaflight / STM32F405 4.2.{patch}\ndefaults nosave\nrateprofile 0\n"
        ));
        assert_eq!(d.number_or_default(&Scope::Rate(0), "roll_expo"), Some(0.0));
        assert!(rates(&d, 0).iter().all(|c| c.points.len() == 201));
    }
    for version in ["4.2.99", "4.2.11.CUSTOM"] {
        let d = config(&format!(
            "# Betaflight / STM32F405 {version}\ndefaults nosave\nrateprofile 0\n"
        ));
        assert!(d.firmware.pack_id.is_some());
        assert!(d.derived.is_empty());
        assert!(rates(&d, 0).iter().all(|c| c.points.is_empty()));
    }
    let d = config("# Betaflight / STM32F405 4.2.11\nrateprofile 0\n");
    assert!(d.derived.is_empty());
}
#[test]
fn declared_craft_and_pilot_names() {
    let d = with("set craft_name = Green Hornet V3\nset pilot_name = papfpv\n");
    assert_eq!(d.craft_name.as_deref(), Some("Green Hornet V3"));
    assert_eq!(d.pilot_name.as_deref(), Some("papfpv"));
    // Betaflight 4.2 and 4.3 have no craft_name setting and print the name
    // only in the dump header, which every supported line writes.
    let d = config("# Betaflight / STM32F745 4.2.11\r\n# name: FLYWOOF7NANO\r\n");
    assert_eq!(d.craft_name.as_deref(), Some("FLYWOOF7NANO"));
    assert_eq!(d.pilot_name, None);
    // A declared setting outranks the header it follows.
    let d = with("# name: Header\nset craft_name = Declared\n");
    assert_eq!(d.craft_name.as_deref(), Some("Declared"));
    // An unset name is absent, not empty, and a reset clears a declared one.
    let d = with("set craft_name = \nset pilot_name =  \n");
    assert_eq!(d.craft_name, None);
    assert_eq!(d.pilot_name, None);
    let d = with("set craft_name = Cleared\ndefaults nosave\n");
    assert_eq!(d.craft_name, None);
    assert_eq!(
        fixture("4.3.0").craft_name.as_deref(),
        Some("FlightLens example")
    );
}
#[test]
fn declared_board_name() {
    let d = with("board_name BETAFPVF4SX1280\n");
    assert_eq!(d.firmware.board_name.as_deref(), Some("BETAFPVF4SX1280"));
    // The `# config:` comment repeats the target, truncated in at least one
    // real backup, and is never read in place of the command line.
    let d = with("# config: manufacturer_id: HOWI, board_name: HOBBYWING_XROTORF7CO\n");
    assert_eq!(d.firmware.board_name, None);
    // A reset does not erase which board the backup came off.
    let d = with("board_name SYNTHETIC\ndefaults nosave\n");
    assert_eq!(d.firmware.board_name.as_deref(), Some("SYNTHETIC"));
    // A longer command that merely starts with the same letters is not one.
    let d = with("board_names FOO\nboard_name\n");
    assert_eq!(d.firmware.board_name, None);
    assert_eq!(
        fixture("4.3.0").firmware.board_name.as_deref(),
        Some("SYNTHETIC")
    );
}

#[test]
fn closing_duplicate_backups_removes_every_saved_path() {
    let temp = tempfile::tempdir().unwrap();
    let mut registry = SourceRegistry::default();
    let mut session = Vec::new();
    let mut duplicate_id = String::new();
    for (name, craft) in [
        ("a.dump", "same"),
        ("b.dump", "same"),
        ("other.dump", "other"),
    ] {
        let path = temp.path().join(name);
        std::fs::write(&path, format!("{}\nset craft_name = {craft}\n", header())).unwrap();
        let source = registry.register(path).unwrap();
        let Artifact::Config(document) =
            open_path(&registry.path(&source.id).unwrap(), &source.id).unwrap()
        else {
            panic!("expected config");
        };
        registry
            .remember_document(&source.id, &document.id)
            .unwrap();
        session.push(registry.path(&source.id).unwrap());
        if craft == "same" {
            if !duplicate_id.is_empty() {
                assert_eq!(duplicate_id, document.id);
            }
            duplicate_id = document.id.clone();
        }
    }
    // Closing uses the imported identity even if a source disappears afterwards.
    std::fs::remove_file(&session[0]).unwrap();
    registry.forget_document(&duplicate_id, &mut session);
    assert_eq!(
        session,
        vec![temp.path().join("other.dump").canonicalize().unwrap()]
    );
    registry.forget_document(&duplicate_id, &mut session);
    assert_eq!(session.len(), 1);
}

#[test]
fn session_identity_follows_successful_reimports_and_recognized_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("backup.dump");
    std::fs::write(&path, format!("{}\nset craft_name = first\n", header())).unwrap();
    let mut registry = SourceRegistry::default();
    let source = registry.register(path.clone()).unwrap();
    let Artifact::Config(first) = open_path(&path, &source.id).unwrap() else {
        panic!("expected config");
    };
    registry.remember_document(&source.id, &first.id).unwrap();
    let mut session = vec![registry.path(&source.id).unwrap()];

    // Reopening a changed path associates it with the new artifact. Closing
    // the old immutable snapshot must not discard the newly opened file.
    std::fs::write(&path, "# INAV / STM32F405 7.0.0\n").unwrap();
    let reopened = registry.register(path.clone()).unwrap();
    let Artifact::Recognized(second) = open_path(&path, &reopened.id).unwrap() else {
        panic!("expected recognized artifact");
    };
    assert_ne!(second.id, reopened.id);
    registry
        .remember_document(&reopened.id, &second.id)
        .unwrap();
    registry.forget_document(&first.id, &mut session);
    assert_eq!(session.len(), 1);
    registry.forget_document(&second.id, &mut session);
    assert!(session.is_empty());
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "# INAV / STM32F405 7.0.0\n"
    );
}

#[test]
fn named_serial_ports_and_numeric_aliases_preserve_identity() {
    let text = "# Betaflight / STM32F405 2025.12.3-alpha.CUSTOM\nserial UART0 1 115200 0 0 0\nserial UART1 1 115200 0 0 0\nserial 0 64 115200 57600 0 115200\nserial 51 262144 115200 0 0 0\nserial soft2 1 115200 0 0 0\nserial PIOUART9 1 115200 0 0 0\n";
    let d = config(text);
    assert_eq!(
        d.syntax.iter().map(|l| l.raw.as_str()).collect::<String>(),
        text
    );
    assert_eq!(d.ports.len(), 4);
    assert_eq!(
        d.ports.iter().map(|p| p.identifier).collect::<Vec<_>>(),
        vec![50, 51, 31, 79]
    );
    assert_eq!(d.ports[0].name, "UART 0");
    assert_eq!(d.ports[1].name, "UART 1");
    assert_eq!(d.ports[1].functions, vec!["Gimbal"]);
    assert!(!d.diagnostics.iter().any(|d| d.severity == "error"));
    let snippet = export(&d, &request(&d, &["serial"])).unwrap();
    assert!(snippet.text.contains("serial UART0 1"));
    assert!(snippet.text.contains("serial UART1 262144"));
    assert_eq!(config(&snippet.text).ports.len(), 4);
    for token in ["UART11", "UART01", "SOFT0", "PIOUART10", "unknown"] {
        assert!(parse_line(&format!("serial {token} 1 115200 0 0 0")).is_err());
    }
    let old = with("serial 0 64 115200 57600 0 115200\n");
    assert_eq!(old.ports[0].identifier, 0);
    assert_eq!(old.ports[0].name, "UART 1");
    assert!(export(&old, &request(&old, &["serial"]))
        .unwrap()
        .text
        .contains("serial 0 64"));
    let invalid_old = with("serial UART0 1 115200 0 0 0\n");
    assert!(export(&invalid_old, &request(&invalid_old, &["serial"])).is_err());
}

#[test]
fn year_based_schema_defaults_and_bounds_are_verified() {
    for patch in 1..=5 {
        let version = format!("2025.12.{patch}");
        let d = config(&format!(
            "# Betaflight / STM32F405 {version}\ndefaults nosave\nrateprofile 0\n"
        ));
        assert!(compatibility::defaults_certified(&version));
        assert_eq!(d.derived["rate:0:thr_hover"].value.cli(), "50");
        assert!(rates(&d, 0).iter().all(|c| c.reason.is_none()));
    }
    for version in [
        "2025.12.3-alpha.KAACK_V19",
        "2025.12.99",
        "2025.12.1+custom",
    ] {
        let d = config(&format!(
            "# Betaflight / STM32F405 {version}\ndefaults nosave\nrateprofile 0\n"
        ));
        assert!(d.firmware.pack_id.is_some());
        assert!(!compatibility::defaults_certified(version));
        assert!(d.derived.is_empty());
        assert!(rates(&d, 0).iter().all(|c| c.points.is_empty()));
    }
    let pack = compatibility::pack(Some("2025.12.3")).unwrap();
    for (key, valid, invalid) in [
        ("align_board_roll", "-180", "-181"),
        ("f_roll", "1000", "1001"),
        ("vtx_channel", "8", "9"),
        ("gps_lap_timer_gate_lat", "-900000000", "900000001"),
        ("ledstrip_race_color", "WHITE", "NOT_A_COLOR"),
    ] {
        let schema = &pack.parameters[key];
        assert!(schema.parse(valid).1, "{key}");
        assert!(!schema.parse(invalid).1, "{key}");
    }
    for key in ["osd_warn_bitmask", "osd_profile", "tpa_low_rate"] {
        assert!(!pack.parameters[key].exportable(), "{key}");
    }
}

#[test]
fn redaction_removes_identifying_and_link_secret_values() {
    let d = config(&format!(
        "{}\n# name: Night Hawk\nset craft_name = Night Hawk\nset pilot_name = Alex\n\
         set expresslrs_uid = 123,45,67,89,10,11\nset srxl2_unit_id = 3\nset motor_poles = 14\n",
        header()
    ));
    let r = feedback::redact(&d);
    for secret in ["Night Hawk", "Alex", "123,45,67,89,10,11"] {
        assert!(!r.text.contains(secret), "{secret} survived redaction");
    }
    // A link identity and an ordinary numeric setting can hold the same value,
    // so the listed key must be removed while the unlisted one is preserved.
    assert!(r.text.contains("set srxl2_unit_id = <redacted>"));
    assert!(r.text.contains("set motor_poles = 14"));
    assert_eq!(
        r.removed
            .iter()
            .map(|x| (x.key.as_str(), x.category))
            .collect::<Vec<_>>(),
        [
            ("name", feedback::Category::Identity),
            ("craft_name", feedback::Category::Identity),
            ("pilot_name", feedback::Category::Identity),
            ("expresslrs_uid", feedback::Category::LinkSecret),
            ("srxl2_unit_id", feedback::Category::LinkSecret),
        ]
    );
    assert!(!r.oversized);
}

#[test]
fn redaction_preserves_every_line_and_its_number() {
    let text = format!(
        "{}\nset craft_name = Hawk\n\n# a comment\nset motor_poles = 14\nnonsense line\n",
        header()
    );
    let d = config(&text);
    let r = feedback::redact(&d);
    assert_eq!(r.text.lines().count(), text.lines().count());
    // A line the parser rejected is the defect a report is most likely about,
    // so it has to survive verbatim.
    assert!(r.text.contains("nonsense line"));
    for x in &r.removed {
        assert_eq!(
            r.text.lines().nth(x.line as usize - 1).unwrap().trim(),
            "set craft_name = <redacted>"
        );
    }
}

#[test]
fn redaction_leaves_a_document_without_secrets_unchanged() {
    let text = format!("{}\nset motor_poles = 14\nfeature OSD\n", header());
    let r = feedback::redact(&config(&text));
    assert_eq!(r.text, text);
    assert!(r.removed.is_empty());
}

#[test]
fn redaction_reports_a_backup_too_large_for_an_issue_body() {
    let filler = "set motor_poles = 14\n".repeat(4000);
    let d = config(&format!("{}\n{filler}", header()));
    assert!(d.syntax.len() > 4000);
    assert!(feedback::redact(&d).oversized);
}

#[test]
fn redaction_covers_the_names_older_firmware_spells_differently() {
    // Betaflight 4.2 and 4.3 write `name` and `display_name` where 4.4 and
    // later write `craft_name` and `pilot_name`, and only 4.2 has the
    // `box_user_*_name` labels. A list built from the newest schema alone
    // leaves the craft name in plain sight in an older backup.
    let d = config(
        "# Betaflight / STM32F405 4.2.0\nset name = Night Hawk\n\
         set display_name = Alex\nset box_user_1_name = Mine\n",
    );
    let r = feedback::redact(&d);
    for secret in ["Night Hawk", "Alex", "Mine"] {
        assert!(!r.text.contains(secret), "{secret} survived redaction");
    }
    assert_eq!(
        r.removed.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(),
        ["name", "display_name", "box_user_1_name"]
    );
}
