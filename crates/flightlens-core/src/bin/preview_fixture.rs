//! Emits only the built-in synthetic fixture for renderer smoke tests, never user input.
use flightlens_core::{analysis, analyze, export, Artifact};
fn main() {
    let artifact = analyze(
        include_str!("../../../../fixtures/configs/betaflight-4.5.0.dump"),
        "Synthetic smoke fixture",
        "virtual",
    )
    .unwrap();
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
    let output = serde_json::json!({"artifact":artifact,"inspection":analysis::inspect(d,0),"snippet":export::export(d,&request).unwrap()});
    println!("{}", serde_json::to_string(&output).unwrap());
}
