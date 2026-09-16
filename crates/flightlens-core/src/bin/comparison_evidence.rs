//! Cross-language regression inputs. Only built-in synthetic text is emitted.
use flightlens_core::{analyze, Artifact};

fn main() {
    let declarations = "profile 0\nrateprofile 0\nset motor_poles = 12\nset motor_poles = 14\nset p_roll = 45\nset roll_rc_rate = 100\nfeature GPS\nfeature -GPS\nserial 0 64 115200 57600 0 115200\naux 0 0 0 900 1200 0 0\nrxrange 0 1000 2000\nrxfail 0 h\nadjrange 0 0 13 1001 1999 34 13\nvtxtable bands 1\nvtxtable channels 1\nvtxtable powerlevels 1\nvtxtable band 1 BAND A CUSTOM 5800\nvtxtable powervalues 25\nvtxtable powerlabels 25\nvtx 0 0 1 1 1 900 1200\n";
    let invalid = "defaults broken\nrxrange broken\nrxfail broken\nadjrange broken\nvtxtable broken\nvtx broken\n";
    let bodies = [
        declarations.to_owned(),
        declarations.replace("1000 2000", "1050 1950").replace("p_roll = 45", "p_roll = 46"),
        format!("{declarations}defaults nosave\n"),
        format!("{declarations}{invalid}"),
        format!("{declarations}{invalid}{declarations}"),
        format!("{declarations}rxrange reset\nrxfail reset\nadjrange reset\nvtxtable bands 0\nvtx reset\n"),
        format!("{declarations}profile 1\nset p_roll = invalid\nrateprofile 1\nset roll_rc_rate = 120\n"),
    ];
    let mut cases = Vec::new();
    for version in ["4.5.0", "99.0.0"] {
        for body in &bodies {
            let text = format!("# Betaflight / STM32F405 (S405) {version}\n# dump all\n\n# synthetic comparison input\n{body}");
            let Artifact::Config(document) =
                analyze(&text, "Synthetic comparison", "virtual").unwrap()
            else {
                panic!("expected synthetic config");
            };
            cases.push(serde_json::json!({"view": document.document_view(), "document": document}));
        }
    }
    println!("{}", serde_json::to_string(&cases).unwrap());
}
