//! Synthetic-only document/IPC probe. No file paths or backup contents accepted.
use flightlens_core::{analyze, Artifact, ArtifactView};
use std::{hint::black_box, io::Write, time::Instant};

fn main() {
    let case = std::env::args()
        .nth(1)
        .expect("case: fixture, 64k, 1m, 16m, settings");
    let compact = std::env::args().nth(2).as_deref() == Some("compact");
    let target = match case.as_str() {
        "fixture" => 0,
        "64k" => 64 * 1024,
        "settings" => 1024 * 1024,
        "1m" => 1024 * 1024,
        "16m" => 16 * 1024 * 1024,
        _ => panic!("case: fixture, 64k, 1m, 16m, settings"),
    };
    let mut text = include_str!("../../../../fixtures/configs/betaflight-4.5.0.dump").to_owned();
    // Fixed 256-byte comments isolate syntax growth from parameter cardinality.
    let line = if case == "settings" {
        "set motor_poles = 14\n".to_owned()
    } else {
        format!("# {}\n", "x".repeat(253))
    };
    while text.len() + line.len() <= target {
        text.push_str(&line);
    }
    let started = Instant::now();
    let artifact = analyze(&text, "Synthetic cost probe", "virtual").unwrap();
    let parse_ms = started.elapsed().as_secs_f64() * 1000.0;
    let Artifact::Config(document) = &artifact else {
        panic!("synthetic fixture must parse as config");
    };
    let started = Instant::now();
    let retained_copy = black_box(document.as_ref().clone());
    let clone_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let payload = if compact {
        serde_json::to_vec(&ArtifactView::Config(Box::new(document.document_view()))).unwrap()
    } else {
        serde_json::to_vec(&artifact).unwrap()
    };
    let serialize_ms = started.elapsed().as_secs_f64() * 1000.0;
    let syntax_bytes = serde_json::to_vec(&document.syntax).unwrap().len();
    let evidence = document.source_evidence();
    let evidence_bytes = serde_json::to_vec(&evidence).unwrap().len();
    eprintln!(
        "{}",
        serde_json::json!({
            "case": case, "format": if compact { "compact" } else { "full" }, "inputBytes": text.len(), "lines": document.syntax.len(),
            "payloadBytes": payload.len(), "syntaxBytes": syntax_bytes,
            "evidenceBytes": evidence_bytes, "evidenceLines": evidence.comparison_syntax.len(),
            "parseMs": parse_ms, "cloneMs": clone_ms, "serializeMs": serialize_ms
        })
    );
    std::io::stdout().lock().write_all(&payload).unwrap();
    black_box(retained_copy);
}
