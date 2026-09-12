//! Emits only the built-in synthetic fixture for renderer smoke tests, never user input.
use flightlens_core::{analysis, analyze, export, Artifact};
fn main() {
    let mut text = include_str!("../../../../fixtures/configs/betaflight-4.5.0.dump").to_owned();
    if std::env::args().any(|arg| arg == "--explicit-only") {
        text = text.replace(
            "defaults nosave",
            "# No reset: omitted values remain unknown",
        );
        text.push_str("\nprofile 1\nset dterm_lpf1_static_hz = 100\n");
    }
    if std::env::args().any(|arg| arg == "--betaflight-42") {
        text = text
            .replace("4.5.0", "4.2.11")
            .replace("MSP API: 1.46", "MSP API: 1.43")
            .replace("dterm_lpf1_static_hz", "dterm_lowpass_hz")
            .replace("dterm_lpf1_type", "dterm_lowpass_type")
            .replace("gyro_lpf1_static_hz", "gyro_lowpass_hz")
            .replace("gyro_lpf1_type", "gyro_lowpass_type");
    }
    if std::env::args().any(|arg| arg == "--cross-version-parameters") {
        text = text
            .replace("4.5.0 Jan", "2025.12.1 Jan")
            .replace("MSP API: 1.46", "MSP API: 1.47")
            .replace("set motor_poles = 14", "set motor_poles = 12");
    }
    if std::env::args().any(|arg| arg == "--parameter-baseline") {
        text.push_str("\nset vbat_max_cell_voltage = 430\nset vbat_min_cell_voltage = 330\nset vbat_warning_cell_voltage = 350\n");
    }
    if std::env::args().any(|arg| arg == "--empty-profiles") {
        text = text
            .replace("profile 1", "profile 3")
            .replace("profile 0", "profile 2");
        text.push_str("\nprofile 0\nrateprofile 0\nprofile 1\nrateprofile 1\n");
    }
    if std::env::args().any(|arg| arg == "--vendor-missing-expo") {
        text = text.replace("4.5.0 Jan", "4.5.3.KAACK_V19 Jan");
        text = text
            .lines()
            .filter(|line| !line.starts_with("set ") || !line.contains("_expo ="))
            .collect::<Vec<_>>()
            .join("\n");
    }
    if std::env::args().any(|arg| arg == "--zero-expo") {
        text = text
            .lines()
            .map(|line| {
                if line.starts_with("set ") && line.contains("_expo =") {
                    format!("{} = 0", line.split(" =").next().unwrap())
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    if std::env::args().any(|arg| arg == "--feature-differences") {
        text.push_str("\nfeature OSD\nfeature -OSD\nfeature 3D\n");
    }
    if std::env::args().any(|arg| arg == "--port-differences") {
        text.push_str("\nserial 0 1 115200 57600 0 115200\nserial 0 64 230400 57600 0 115200\nserial 1 0 115200 57600 0 115200\n");
    }
    if std::env::args().any(|arg| arg == "--mode-differences") {
        text.push_str(
            "\naux 0 0 0 1700 2100 0 0\naux 0 0 1 1600 2100 1 0\naux 2 2 14 900 900 0 0\n",
        );
    }
    if std::env::args().any(|arg| arg == "--vtxtable-baseline" || arg == "--vtxtable-differences") {
        text.push_str("\nvtxtable bands 1\nvtxtable channels 2\nvtxtable band 1 RACE R FACTORY 5658 5695\nvtxtable powerlevels 2\nvtxtable powervalues 25 100\nvtxtable powerlabels 25 100\n");
    }
    if std::env::args().any(|arg| arg == "--vtxtable-differences") {
        text.push_str("\nvtxtable band 1 race r factory 05658 5695\nvtxtable powerlevels 1\nvtxtable powerlevels 2\nvtxtable powervalues 25 200\nvtxtable\n");
    }
    if std::env::args().any(|arg| arg == "--rxfail-baseline") {
        text.push_str("\nrxfail 0 s 1001\nrxfail 1 a\nrxfail 3 h\n");
    }
    if std::env::args().any(|arg| arg == "--rxfail-differences") {
        text.push_str(
            "\nrxfail 0 h\nrxfail 0 s 1024\nrxfail\nrxfail 0\nrxfail 1 h\nrxfail 17 s 2250\n",
        );
    }
    if std::env::args().any(|arg| arg == "--collection-baseline") {
        text.push_str("\nvtxtable bands 5\nrxrange 0 1000 2000\n");
    }
    if std::env::args().any(|arg| arg == "--collection-differences") {
        text.push_str("\nrxrange 0 1000 2000\nrxrange 0 1050 1950\nadjrange 0 0 0 900 900 0 0\n");
    }
    let artifact = analyze(&text, "Synthetic smoke fixture", "virtual").unwrap();
    let Artifact::Config(d) = &artifact else {
        unreachable!()
    };
    let request = export::ExportRequest {
        groups: vec!["rates".into()],
        pid_profile: 0,
        rate_profile: 0,
        destination_header: d.firmware.header.clone().unwrap(),
        include_save: false,
    };
    let inspections: std::collections::BTreeMap<_, _> = d
        .rate_profiles
        .iter()
        .map(|profile| (*profile, analysis::inspect(d, *profile)))
        .collect();
    let output = serde_json::json!({"artifact":artifact,"inspection":analysis::inspect(d,0),"inspections":inspections,"snippet":export::export(d,&request).ok()});
    println!("{}", serde_json::to_string(&output).unwrap());
}
